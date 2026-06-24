/// Privileged operation types and executor trait.
///
/// Security contract:
/// - `SudoSystemctl::run` ALWAYS checks the allowlist BEFORE executing any command.
/// - The Restart command is a fixed argv: `sudo -n systemctl restart <unit>` — no shell, no interpolation.
/// - A non-allowlisted unit returns `Err` immediately; nothing is executed.
/// - The Upgrade command validates the version string via `version::validate` BEFORE constructing
///   any command. The validated canonical form is passed as a single arg to the upgrade script.

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Restart { unit: String },
    Upgrade { version: String },
}

pub trait OpExecutor: Send + Sync {
    fn run(&self, op: &Op) -> anyhow::Result<String>;
}

/// Real executor: runs privileged commands via sudo with no shell.
/// `allowed` is the allowlist of unit names that may be restarted.
/// `upgrade_script` is the path to the operator-installed upgrade wrapper.
pub struct SudoSystemctl {
    pub allowed: Vec<String>,
    pub upgrade_script: String,
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
            Op::Upgrade { version } => {
                // Validate BEFORE constructing any command — defense in depth.
                // version::validate rejects anything that isn't a clean semver triple.
                let canon = crate::version::validate(version)
                    .ok_or_else(|| anyhow::anyhow!("invalid upgrade version: {version}"))?;
                // Fixed argv: sudo -n <upgrade_script> <canonical_version>
                // No sh -c, no shell expansion; canonical is a bare "x.y.z" string.
                let out = std::process::Command::new("sudo")
                    .args(["-n", self.upgrade_script.as_str(), canon.as_str()])
                    .output()?;
                if !out.status.success() {
                    anyhow::bail!(
                        "upgrade to {canon} failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Ok(format!("upgraded to {canon}"))
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
                Op::Upgrade { version } => Ok(format!("fake-upgraded {version}")),
            }
        }
    }

    #[test]
    fn sudo_systemctl_rejects_unlisted_unit() {
        let exec = SudoSystemctl { allowed: vec!["monad-bft.service".into()], upgrade_script: "/usr/local/bin/monad-upgrade.sh".into() };
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

    #[test]
    fn recording_executor_records_upgrade() {
        let exec = RecordingExecutor::new();
        exec.run(&Op::Upgrade { version: "0.14.7".into() }).unwrap();
        let calls = exec.calls.lock().unwrap();
        assert_eq!(*calls, vec![Op::Upgrade { version: "0.14.7".into() }]);
    }

    #[test]
    fn sudo_upgrade_rejects_bad_version_before_exec() {
        let s = SudoSystemctl { allowed: vec![], upgrade_script: "/usr/local/bin/monad-upgrade.sh".into() };
        // a malformed version must never reach the command
        let err = s.run(&Op::Upgrade { version: "0.14.5; rm -rf /".into() }).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("version"));
    }
}
