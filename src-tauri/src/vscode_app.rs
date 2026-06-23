use crate::switch_transaction::{
    RestorePlan, SwitchTransaction, TransactionError, TransactionPhase, TransactionRunner,
};
use serde::{Deserialize, Serialize};
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

const VSCODE_PROCESS_NAMES: &[&str] = &["Visual Studio Code", "Code", "Code - Insiders"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VscodePostSwitchAction {
    ManualReloadWindow,
    RestartApp,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VscodeSwitchOptions {
    pub app_path: Option<String>,
    pub post_switch_action: VscodePostSwitchAction,
    pub quit_timeout_ms: u64,
}

impl Default for VscodeSwitchOptions {
    fn default() -> Self {
        Self {
            app_path: None,
            post_switch_action: VscodePostSwitchAction::ManualReloadWindow,
            quit_timeout_ms: 8_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VscodeSwitchReport {
    pub was_running: bool,
    pub quit_requested: bool,
    pub restart_requested: bool,
    pub manual_action: Option<String>,
    pub transaction: SwitchTransaction,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VscodeAppError {
    Process(String),
    QuitTimedOut(Vec<String>),
    RestartUnavailable,
    Transaction(TransactionError),
}

impl From<TransactionError> for VscodeAppError {
    fn from(value: TransactionError) -> Self {
        VscodeAppError::Transaction(value)
    }
}

pub trait VscodeProcessController {
    fn running_processes(&self) -> Result<Vec<String>, VscodeAppError>;
    fn request_quit(&self, process_names: &[&str]) -> Result<(), VscodeAppError>;
    fn restart(&self, app_path: Option<&str>) -> Result<(), VscodeAppError>;
}

pub struct MacVscodeProcessController;

impl VscodeProcessController for MacVscodeProcessController {
    fn running_processes(&self) -> Result<Vec<String>, VscodeAppError> {
        let mut found = Vec::new();
        for name in VSCODE_PROCESS_NAMES {
            let output = Command::new("pgrep")
                .args(["-x", name])
                .output()
                .map_err(|error| VscodeAppError::Process(error.to_string()))?;
            if output.status.success() {
                found.push((*name).to_string());
            }
        }
        Ok(found)
    }

    fn request_quit(&self, process_names: &[&str]) -> Result<(), VscodeAppError> {
        for name in process_names {
            let script = format!("tell application \"{name}\" to quit");
            let output = Command::new("osascript")
                .args(["-e", &script])
                .output()
                .map_err(|error| VscodeAppError::Process(error.to_string()))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !stderr.contains("Can’t get application") && !stderr.contains("Application isn’t running") {
                    return Err(VscodeAppError::Process(stderr));
                }
            }
        }
        Ok(())
    }

    fn restart(&self, app_path: Option<&str>) -> Result<(), VscodeAppError> {
        let Some(app_path) = app_path else {
            return Err(VscodeAppError::RestartUnavailable);
        };
        let output = Command::new("open")
            .arg(app_path)
            .output()
            .map_err(|error| VscodeAppError::Process(error.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(VscodeAppError::Process(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }
}

pub struct VscodeSwitchCoordinator<C: VscodeProcessController> {
    process_controller: C,
    transaction_runner: TransactionRunner,
}

impl<C: VscodeProcessController> VscodeSwitchCoordinator<C> {
    pub fn new(process_controller: C, transaction_runner: TransactionRunner) -> Self {
        Self {
            process_controller,
            transaction_runner,
        }
    }

    pub fn switch_vscode_profile(
        &self,
        plan: &RestorePlan,
        options: &VscodeSwitchOptions,
    ) -> Result<VscodeSwitchReport, VscodeAppError> {
        let was_running = !self.process_controller.running_processes()?.is_empty();
        let transaction = self.transaction_runner.run(plan)?;
        let mut warnings = Vec::new();
        let mut quit_requested = false;
        let mut restart_requested = false;
        let mut manual_action = None;

        if transaction.phase != TransactionPhase::Completed {
            warnings.push("Restore did not complete; VS Code reload or restart skipped".to_string());
            return Ok(VscodeSwitchReport {
                was_running,
                quit_requested,
                restart_requested,
                manual_action,
                transaction,
                warnings,
            });
        }

        match options.post_switch_action {
            VscodePostSwitchAction::ManualReloadWindow => {
                manual_action = Some(
                    "In VS Code, run Developer: Reload Window after saving any unsaved work".to_string(),
                );
            }
            VscodePostSwitchAction::RestartApp => {
                if was_running {
                    self.process_controller.request_quit(VSCODE_PROCESS_NAMES)?;
                    quit_requested = true;
                    self.wait_until_stopped(Duration::from_millis(options.quit_timeout_ms))?;
                }
                self.process_controller.restart(options.app_path.as_deref())?;
                restart_requested = true;
            }
            VscodePostSwitchAction::None => {
                warnings.push("VS Code restart/reload was skipped by configuration".to_string());
            }
        }

        Ok(VscodeSwitchReport {
            was_running,
            quit_requested,
            restart_requested,
            manual_action,
            transaction,
            warnings,
        })
    }

    fn wait_until_stopped(&self, timeout: Duration) -> Result<(), VscodeAppError> {
        let start = Instant::now();
        loop {
            let running = self.process_controller.running_processes()?;
            if running.is_empty() {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(VscodeAppError::QuitTimedOut(running));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{
        cell::RefCell,
        fs,
        path::PathBuf,
        rc::Rc,
    };

    #[derive(Default)]
    struct MockState {
        running: bool,
        quit_requested: usize,
        restart_requested: usize,
        never_stops: bool,
    }

    #[derive(Clone, Default)]
    struct MockController {
        state: Rc<RefCell<MockState>>,
    }

    impl MockController {
        fn running() -> Self {
            let controller = Self::default();
            controller.state.borrow_mut().running = true;
            controller
        }
    }

    impl VscodeProcessController for MockController {
        fn running_processes(&self) -> Result<Vec<String>, VscodeAppError> {
            if self.state.borrow().running {
                Ok(vec!["Visual Studio Code".to_string()])
            } else {
                Ok(Vec::new())
            }
        }

        fn request_quit(&self, _process_names: &[&str]) -> Result<(), VscodeAppError> {
            let mut state = self.state.borrow_mut();
            state.quit_requested += 1;
            if !state.never_stops {
                state.running = false;
            }
            Ok(())
        }

        fn restart(&self, _app_path: Option<&str>) -> Result<(), VscodeAppError> {
            let mut state = self.state.borrow_mut();
            state.restart_requested += 1;
            state.running = true;
            Ok(())
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-vscode-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn restore_plan(root: &std::path::Path, content: &str) -> RestorePlan {
        RestorePlan {
            transaction_id: "vscode-tx".to_string(),
            target_profile_id: "profile-vscode".to_string(),
            artifacts: vec![crate::switch_transaction::RestoreArtifact {
                environment: "vscode".to_string(),
                target_path: root.join("Code/User/globalStorage/openai.codex/state.json"),
                content_base64: STANDARD.encode(content.as_bytes()),
            }],
        }
    }

    #[test]
    fn manual_reload_action_restores_without_restarting() {
        let root = temp_dir("manual");
        let controller = MockController::running();
        let state = controller.state.clone();
        let coordinator =
            VscodeSwitchCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_vscode_profile(
                &restore_plan(&root, "new"),
                &VscodeSwitchOptions {
                    app_path: None,
                    post_switch_action: VscodePostSwitchAction::ManualReloadWindow,
                    quit_timeout_ms: 50,
                },
            )
            .expect("switch vscode");

        assert_eq!(report.transaction.phase, TransactionPhase::Completed);
        assert!(report.manual_action.expect("manual action").contains("Reload Window"));
        assert_eq!(state.borrow().quit_requested, 0);
        assert_eq!(state.borrow().restart_requested, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_action_quits_and_reopens_vscode() {
        let root = temp_dir("restart");
        let controller = MockController::running();
        let state = controller.state.clone();
        let coordinator =
            VscodeSwitchCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_vscode_profile(
                &restore_plan(&root, "new"),
                &VscodeSwitchOptions {
                    app_path: Some("/Applications/Visual Studio Code.app".to_string()),
                    post_switch_action: VscodePostSwitchAction::RestartApp,
                    quit_timeout_ms: 50,
                },
            )
            .expect("switch vscode with restart");

        assert!(report.quit_requested);
        assert!(report.restart_requested);
        assert_eq!(state.borrow().quit_requested, 1);
        assert_eq!(state.borrow().restart_requested, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_timeout_reports_running_processes() {
        let root = temp_dir("timeout");
        let controller = MockController::running();
        controller.state.borrow_mut().never_stops = true;
        let coordinator =
            VscodeSwitchCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let error = coordinator
            .switch_vscode_profile(
                &restore_plan(&root, "new"),
                &VscodeSwitchOptions {
                    app_path: Some("/Applications/Visual Studio Code.app".to_string()),
                    post_switch_action: VscodePostSwitchAction::RestartApp,
                    quit_timeout_ms: 1,
                },
            )
            .expect_err("quit should time out");

        assert_eq!(
            error,
            VscodeAppError::QuitTimedOut(vec!["Visual Studio Code".to_string()])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_failure_skips_reload_or_restart() {
        let root = temp_dir("restore-failure");
        let controller = MockController::running();
        let state = controller.state.clone();
        let mut plan = restore_plan(&root, "new");
        plan.artifacts[0].content_base64 = "not-base64".to_string();
        let coordinator =
            VscodeSwitchCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_vscode_profile(
                &plan,
                &VscodeSwitchOptions {
                    app_path: Some("/Applications/Visual Studio Code.app".to_string()),
                    post_switch_action: VscodePostSwitchAction::RestartApp,
                    quit_timeout_ms: 50,
                },
            )
            .expect("restore failure should be reported");

        assert_eq!(report.transaction.phase, TransactionPhase::Failed);
        assert_eq!(state.borrow().quit_requested, 0);
        assert_eq!(state.borrow().restart_requested, 0);
        assert!(!report.warnings.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}

