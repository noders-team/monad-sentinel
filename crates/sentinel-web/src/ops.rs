/// Privileged operation types and executor trait.
///
/// Security contract:
/// - `SudoSystemctl::run` ALWAYS checks the allowlist BEFORE executing any command.
/// - The command is a fixed argv: `sudo -n systemctl restart <unit>` — no shell, no interpolation.
/// - A non-allowlisted unit returns `Err` immediately; nothing is executed.

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Restart { unit: String },
}

pub trait OpExecutor: Send + Sync {
    fn run(&self, op: &Op) -> anyhow::Result<String>;
}

/// Real executor: runs `sudo -n systemctl restart <unit>` with no shell.
/// `allowed` is the allowlist of unit names that may be touched.
pub struct SudoSystemctl {
    pub allowed: Vec<String>,
}

impl OpExecutor for SudoSystemctl {
    fn run(&self, op: &Op) -> anyhow::Result<String> {
        match op {
            Op::Restart { unit } => {
                // Allowlist check — MUST happen before any exec.
                if !self.allowed.iter().any(|u| u == unit) {
                    anyhow::bail!("unit not allowed: {unit}");
                }
                // Fixed argv — no sh -c, no string interpolation in the shell sense.
                let out = std::process::Command::new("sudo")
                    .args(["-n", "systemctl", "restart", unit.as_str()])
                    .output()?;
                if !out.status.success() {
                    anyhow::bail!(
                        "systemctl restart {unit} failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Ok(format!("restarted {unit}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingExecutor {
        calls: std::sync::Mutex<Vec<Op>>,
    }
    impl RecordingExecutor {
        fn new() -> Self { Self { calls: std::sync::Mutex::new(vec![]) } }
    }
    impl OpExecutor for RecordingExecutor {
        fn run(&self, op: &Op) -> anyhow::Result<String> {
            self.calls.lock().unwrap().push(op.clone());
            match op {
                Op::Restart { unit } => Ok(format!("fake-restarted {unit}")),
            }
        }
    }

    #[test]
    fn sudo_systemctl_rejects_unlisted_unit() {
        let exec = SudoSystemctl { allowed: vec!["monad-bft.service".into()] };
        let err = exec.run(&Op::Restart { unit: "evil.service".into() }).unwrap_err();
        assert!(err.to_string().contains("not allowed"));
    }

    #[test]
    fn recording_executor_captures_ops() {
        let exec = RecordingExecutor::new();
        exec.run(&Op::Restart { unit: "monad-bft.service".into() }).unwrap();
        let calls = exec.calls.lock().unwrap();
        assert_eq!(*calls, vec![Op::Restart { unit: "monad-bft.service".into() }]);
    }
}
