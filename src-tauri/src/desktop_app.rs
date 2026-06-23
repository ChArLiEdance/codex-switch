use crate::switch_transaction::{
    RestorePlan, SwitchTransaction, TransactionError, TransactionPhase, TransactionRunner,
};
use serde::{Deserialize, Serialize};
use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

const DESKTOP_PROCESS_NAMES: &[&str] = &["Codex", "Codex Desktop", "OpenAI Codex"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSwitchOptions {
    pub app_path: Option<String>,
    pub auto_restart: bool,
    pub quit_timeout_ms: u64,
}

impl Default for DesktopSwitchOptions {
    fn default() -> Self {
        Self {
            app_path: None,
            auto_restart: true,
            quit_timeout_ms: 8_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSwitchReport {
    pub was_running: bool,
    pub quit_requested: bool,
    pub restart_requested: bool,
    pub transaction: SwitchTransaction,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopAppError {
    Process(String),
    QuitTimedOut(Vec<String>),
    RestartTimedOut,
    RestartUnavailable,
    Transaction(TransactionError),
}

impl From<TransactionError> for DesktopAppError {
    fn from(value: TransactionError) -> Self {
        DesktopAppError::Transaction(value)
    }
}

pub trait DesktopProcessController {
    fn running_processes(&self) -> Result<Vec<String>, DesktopAppError>;
    fn request_quit(&self, process_names: &[&str]) -> Result<(), DesktopAppError>;
    fn restart(&self, app_path: Option<&str>) -> Result<(), DesktopAppError>;
}

pub struct MacDesktopProcessController;

impl DesktopProcessController for MacDesktopProcessController {
    fn running_processes(&self) -> Result<Vec<String>, DesktopAppError> {
        let mut found = Vec::new();
        for name in DESKTOP_PROCESS_NAMES {
            let output = Command::new("pgrep")
                .args(["-x", name])
                .output()
                .map_err(|error| DesktopAppError::Process(error.to_string()))?;
            if output.status.success() {
                found.push((*name).to_string());
            }
        }
        Ok(found)
    }

    fn request_quit(&self, process_names: &[&str]) -> Result<(), DesktopAppError> {
        for name in process_names {
            let script = format!("tell application \"{name}\" to quit");
            let output = Command::new("osascript")
                .args(["-e", &script])
                .output()
                .map_err(|error| DesktopAppError::Process(error.to_string()))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if !stderr.contains("Can’t get application")
                    && !stderr.contains("Application isn’t running")
                {
                    return Err(DesktopAppError::Process(stderr));
                }
            }
        }
        Ok(())
    }

    fn restart(&self, app_path: Option<&str>) -> Result<(), DesktopAppError> {
        let Some(app_path) = app_path else {
            return Err(DesktopAppError::RestartUnavailable);
        };
        let output = Command::new("open")
            .arg(app_path)
            .output()
            .map_err(|error| DesktopAppError::Process(error.to_string()))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(DesktopAppError::Process(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ))
        }
    }
}

pub struct DesktopAppCoordinator<C: DesktopProcessController> {
    process_controller: C,
    transaction_runner: TransactionRunner,
}

impl<C: DesktopProcessController> DesktopAppCoordinator<C> {
    pub fn new(process_controller: C, transaction_runner: TransactionRunner) -> Self {
        Self {
            process_controller,
            transaction_runner,
        }
    }

    pub fn switch_desktop_profile(
        &self,
        plan: &RestorePlan,
        options: &DesktopSwitchOptions,
    ) -> Result<DesktopSwitchReport, DesktopAppError> {
        let running = self.process_controller.running_processes()?;
        let was_running = !running.is_empty();
        let mut warnings = Vec::new();
        let mut quit_requested = false;
        let mut restart_requested = false;

        if was_running {
            self.process_controller
                .request_quit(DESKTOP_PROCESS_NAMES)?;
            quit_requested = true;
            self.wait_until_stopped(Duration::from_millis(options.quit_timeout_ms))?;
        }

        let transaction = self.transaction_runner.run(plan)?;
        if transaction.phase != TransactionPhase::Completed {
            warnings.push("Restore did not complete; restart skipped".to_string());
            return Ok(DesktopSwitchReport {
                was_running,
                quit_requested,
                restart_requested,
                transaction,
                warnings,
            });
        }

        if options.auto_restart {
            self.process_controller
                .restart(options.app_path.as_deref())?;
            self.wait_until_started(Duration::from_millis(options.quit_timeout_ms))?;
            restart_requested = true;
        }

        Ok(DesktopSwitchReport {
            was_running,
            quit_requested,
            restart_requested,
            transaction,
            warnings,
        })
    }

    fn wait_until_stopped(&self, timeout: Duration) -> Result<(), DesktopAppError> {
        let start = Instant::now();
        loop {
            let running = self.process_controller.running_processes()?;
            if running.is_empty() {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(DesktopAppError::QuitTimedOut(running));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_until_started(&self, timeout: Duration) -> Result<(), DesktopAppError> {
        let start = Instant::now();
        loop {
            let running = self.process_controller.running_processes()?;
            if !running.is_empty() {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(DesktopAppError::RestartTimedOut);
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

    #[derive(Default)]
    struct MockState {
        running: bool,
        quit_requested: usize,
        restart_requested: usize,
        fail_restart: bool,
        restart_does_not_start: bool,
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

    impl DesktopProcessController for MockController {
        fn running_processes(&self) -> Result<Vec<String>, DesktopAppError> {
            if self.state.borrow().running {
                Ok(vec!["Codex".to_string()])
            } else {
                Ok(Vec::new())
            }
        }

        fn request_quit(&self, _process_names: &[&str]) -> Result<(), DesktopAppError> {
            let mut state = self.state.borrow_mut();
            state.quit_requested += 1;
            if !state.never_stops {
                state.running = false;
            }
            Ok(())
        }

        fn restart(&self, _app_path: Option<&str>) -> Result<(), DesktopAppError> {
            let mut state = self.state.borrow_mut();
            state.restart_requested += 1;
            if state.fail_restart {
                Err(DesktopAppError::Process("restart failed".to_string()))
            } else {
                state.running = !state.restart_does_not_start;
                Ok(())
            }
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "codex-switch-desktop-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn restore_plan(root: &std::path::Path, content: &str) -> RestorePlan {
        RestorePlan {
            transaction_id: "desktop-tx".to_string(),
            target_profile_id: "profile-1".to_string(),
            artifacts: vec![crate::switch_transaction::RestoreArtifact {
                environment: "desktop".to_string(),
                kind: crate::switch_transaction::RestoreArtifactKind::Config,
                target_path: root.join("Codex/config.json"),
                content_base64: STANDARD.encode(content.as_bytes()),
                unix_mode: None,
            }],
        }
    }

    #[test]
    fn closes_restores_and_restarts_desktop_app() {
        let root = temp_dir("success");
        let target = root.join("Codex/config.json");
        fs::create_dir_all(target.parent().expect("parent")).expect("create parent");
        fs::write(&target, "old").expect("write old");
        let controller = MockController::running();
        let state = controller.state.clone();
        let coordinator =
            DesktopAppCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_desktop_profile(
                &restore_plan(&root, "new"),
                &DesktopSwitchOptions {
                    app_path: Some("/Applications/Codex.app".to_string()),
                    auto_restart: true,
                    quit_timeout_ms: 50,
                },
            )
            .expect("switch desktop profile");

        assert!(report.was_running);
        assert!(report.quit_requested);
        assert!(report.restart_requested);
        assert_eq!(report.transaction.phase, TransactionPhase::Completed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "new");
        assert_eq!(state.borrow().quit_requested, 1);
        assert_eq!(state.borrow().restart_requested, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_is_skipped_when_auto_restart_disabled() {
        let root = temp_dir("no-restart");
        let controller = MockController::default();
        let state = controller.state.clone();
        let coordinator =
            DesktopAppCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_desktop_profile(
                &restore_plan(&root, "new"),
                &DesktopSwitchOptions {
                    app_path: None,
                    auto_restart: false,
                    quit_timeout_ms: 50,
                },
            )
            .expect("switch without restart");

        assert!(!report.restart_requested);
        assert_eq!(state.borrow().restart_requested, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn quit_timeout_reports_running_processes() {
        let root = temp_dir("timeout");
        let controller = MockController::running();
        controller.state.borrow_mut().never_stops = true;
        let coordinator =
            DesktopAppCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let error = coordinator
            .switch_desktop_profile(
                &restore_plan(&root, "new"),
                &DesktopSwitchOptions {
                    app_path: Some("/Applications/Codex.app".to_string()),
                    auto_restart: true,
                    quit_timeout_ms: 1,
                },
            )
            .expect_err("quit should time out");

        assert_eq!(
            error,
            DesktopAppError::QuitTimedOut(vec!["Codex".to_string()])
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restart_timeout_reports_unstarted_desktop() {
        let root = temp_dir("restart-timeout");
        let controller = MockController::default();
        controller.state.borrow_mut().restart_does_not_start = true;
        let coordinator =
            DesktopAppCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let error = coordinator
            .switch_desktop_profile(
                &restore_plan(&root, "new"),
                &DesktopSwitchOptions {
                    app_path: Some("/Applications/Codex.app".to_string()),
                    auto_restart: true,
                    quit_timeout_ms: 1,
                },
            )
            .expect_err("restart should time out");

        assert_eq!(error, DesktopAppError::RestartTimedOut);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restore_failure_rolls_back_and_skips_restart() {
        let root = temp_dir("restore-failure");
        let target = root.join("Codex/config.json");
        fs::create_dir_all(target.parent().expect("parent")).expect("create parent");
        fs::write(&target, "old").expect("write old");
        let controller = MockController::default();
        let state = controller.state.clone();
        let mut plan = restore_plan(&root, "new");
        plan.artifacts[0].content_base64 = "not-base64".to_string();
        let coordinator =
            DesktopAppCoordinator::new(controller, TransactionRunner::new(root.join("backups")));

        let report = coordinator
            .switch_desktop_profile(
                &plan,
                &DesktopSwitchOptions {
                    app_path: Some("/Applications/Codex.app".to_string()),
                    auto_restart: true,
                    quit_timeout_ms: 50,
                },
            )
            .expect("transaction failure is reported without restart");

        assert_eq!(report.transaction.phase, TransactionPhase::Failed);
        assert_eq!(fs::read_to_string(target).expect("read target"), "old");
        assert_eq!(state.borrow().restart_requested, 0);
        assert!(!report.warnings.is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
