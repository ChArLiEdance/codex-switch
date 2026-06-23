use crate::switch_transaction::{
    RestorePlan, SwitchTransaction, TransactionError, TransactionPhase, TransactionRunner,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliValidationStatus {
    Verified,
    Inconclusive,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliValidation {
    pub status: CliValidationStatus,
    pub command: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliSwitchReport {
    pub transaction: SwitchTransaction,
    pub validation: CliValidation,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliSwitchError {
    CliTaskRunning(Vec<String>),
    Runtime(String),
    Transaction(TransactionError),
}

impl From<TransactionError> for CliSwitchError {
    fn from(value: TransactionError) -> Self {
        CliSwitchError::Transaction(value)
    }
}

pub trait CliRuntime {
    fn running_tasks(&self) -> Result<Vec<String>, CliSwitchError>;
    fn validate(&self) -> Result<CliValidation, CliSwitchError>;
}

pub struct SystemCliRuntime {
    codex_path: Option<String>,
}

impl SystemCliRuntime {
    pub fn new(codex_path: Option<String>) -> Self {
        Self { codex_path }
    }

    fn command_label(&self) -> String {
        self.codex_path
            .clone()
            .unwrap_or_else(|| "codex".to_string())
    }
}

impl CliRuntime for SystemCliRuntime {
    fn running_tasks(&self) -> Result<Vec<String>, CliSwitchError> {
        #[cfg(windows)]
        let output = Command::new("tasklist")
            .output()
            .map_err(|error| CliSwitchError::Runtime(error.to_string()))?;
        #[cfg(not(windows))]
        let output = Command::new("pgrep")
            .args(["-fl", "codex"])
            .output()
            .map_err(|error| CliSwitchError::Runtime(error.to_string()))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter(|line| !line.contains("codex_switch"))
            .map(ToOwned::to_owned)
            .collect())
    }

    fn validate(&self) -> Result<CliValidation, CliSwitchError> {
        let command = format!("{} --version", self.command_label());
        let output = Command::new(self.command_label()).arg("--version").output();

        match output {
            Ok(output) if output.status.success() => Ok(CliValidation {
                status: CliValidationStatus::Inconclusive,
                command,
                message:
                    "Codex CLI responded, but account identity verification is not implemented yet"
                        .to_string(),
            }),
            Ok(output) => Ok(CliValidation {
                status: CliValidationStatus::Failed,
                command,
                message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            }),
            Err(error) => Ok(CliValidation {
                status: CliValidationStatus::Failed,
                command,
                message: error.to_string(),
            }),
        }
    }
}

pub struct CliSwitchCoordinator<R: CliRuntime> {
    runtime: R,
    transaction_runner: TransactionRunner,
}

impl<R: CliRuntime> CliSwitchCoordinator<R> {
    pub fn new(runtime: R, transaction_runner: TransactionRunner) -> Self {
        Self {
            runtime,
            transaction_runner,
        }
    }

    pub fn switch_cli_profile(
        &self,
        plan: &RestorePlan,
    ) -> Result<CliSwitchReport, CliSwitchError> {
        let running_tasks = self.runtime.running_tasks()?;
        if !running_tasks.is_empty() {
            return Err(CliSwitchError::CliTaskRunning(running_tasks));
        }

        let transaction = self.transaction_runner.run(plan)?;
        let mut warnings = Vec::new();
        if transaction.phase != TransactionPhase::Completed {
            warnings.push("Restore did not complete; CLI validation skipped".to_string());
            return Ok(CliSwitchReport {
                transaction,
                validation: CliValidation {
                    status: CliValidationStatus::Failed,
                    command: "codex --version".to_string(),
                    message: "Restore failed and rollback was attempted".to_string(),
                },
                warnings,
            });
        }

        let validation = self.runtime.validate()?;
        if validation.status == CliValidationStatus::Inconclusive {
            warnings.push(
                "Configuration restored, but CLI account identity is not confirmed".to_string(),
            );
        }

        Ok(CliSwitchReport {
            transaction,
            validation,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

    #[derive(Clone)]
    struct MockCliRuntime {
        state: Rc<RefCell<MockCliState>>,
    }

    struct MockCliState {
        running_tasks: Vec<String>,
        validation: CliValidation,
        validations: usize,
    }

    impl MockCliRuntime {
        fn new(validation_status: CliValidationStatus) -> Self {
            Self {
                state: Rc::new(RefCell::new(MockCliState {
                    running_tasks: Vec::new(),
                    validation: CliValidation {
                        status: validation_status,
                        command: "codex --version".to_string(),
                        message: "mock validation".to_string(),
                    },
                    validations: 0,
                })),
            }
        }

        fn with_running_task(task: &str) -> Self {
            let runtime = Self::new(CliValidationStatus::Verified);
            runtime
                .state
                .borrow_mut()
                .running_tasks
                .push(task.to_string());
            runtime
        }
    }

    impl CliRuntime for MockCliRuntime {
        fn running_tasks(&self) -> Result<Vec<String>, CliSwitchError> {
            Ok(self.state.borrow().running_tasks.clone())
        }

        fn validate(&self) -> Result<CliValidation, CliSwitchError> {
            let mut state = self.state.borrow_mut();
            state.validations += 1;
            Ok(state.validation.clone())
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("codex-switch-cli-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn restore_plan(root: &std::path::Path, content: &str) -> RestorePlan {
        RestorePlan {
            transaction_id: "cli-tx".to_string(),
            target_profile_id: "profile-cli".to_string(),
            artifacts: vec![crate::switch_transaction::RestoreArtifact {
                environment: "cli".to_string(),
                target_path: root.join(".codex/auth.json"),
                content_base64: STANDARD.encode(content.as_bytes()),
            }],
        }
    }

    #[test]
    fn cli_switch_restores_and_validates_immediately() {
        let root = temp_dir("success");
        let target = root.join(".codex/auth.json");
        fs::create_dir_all(target.parent().expect("parent")).expect("create parent");
        fs::write(&target, "old").expect("write old");
        let runtime = MockCliRuntime::new(CliValidationStatus::Verified);
        let state = runtime.state.clone();
        let coordinator =
            CliSwitchCoordinator::new(runtime, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_cli_profile(&restore_plan(&root, "new"))
            .expect("switch cli");

        assert_eq!(report.transaction.phase, TransactionPhase::Completed);
        assert_eq!(report.validation.status, CliValidationStatus::Verified);
        assert_eq!(fs::read_to_string(target).expect("read target"), "new");
        assert_eq!(state.borrow().validations, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cli_switch_blocks_when_cli_task_is_running() {
        let root = temp_dir("running");
        let runtime = MockCliRuntime::with_running_task("123 codex exec task");
        let coordinator =
            CliSwitchCoordinator::new(runtime, TransactionRunner::new(root.join("backups")));

        let error = coordinator
            .switch_cli_profile(&restore_plan(&root, "new"))
            .expect_err("running task should block switch");

        assert_eq!(
            error,
            CliSwitchError::CliTaskRunning(vec!["123 codex exec task".to_string()])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inconclusive_validation_reports_warning() {
        let root = temp_dir("inconclusive");
        let runtime = MockCliRuntime::new(CliValidationStatus::Inconclusive);
        let coordinator =
            CliSwitchCoordinator::new(runtime, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_cli_profile(&restore_plan(&root, "new"))
            .expect("switch cli");

        assert_eq!(report.validation.status, CliValidationStatus::Inconclusive);
        assert_eq!(report.warnings.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_failure_skips_cli_validation() {
        let root = temp_dir("restore-failure");
        let runtime = MockCliRuntime::new(CliValidationStatus::Verified);
        let state = runtime.state.clone();
        let mut plan = restore_plan(&root, "new");
        plan.artifacts[0].content_base64 = "not-base64".to_string();
        let coordinator =
            CliSwitchCoordinator::new(runtime, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_cli_profile(&plan)
            .expect("restore failure should report transaction");

        assert_eq!(report.transaction.phase, TransactionPhase::Failed);
        assert_eq!(state.borrow().validations, 0);
        assert!(!report.warnings.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
