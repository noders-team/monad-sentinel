pub mod telegram;

use crate::alert::{Severity, Transition};
use crate::rules::Fired;

pub trait Notifier {
    fn send(&self, text: &str) -> anyhow::Result<()>;
}

pub fn format_message(f: &Fired) -> String {
    let sev = match f.severity {
        Severity::Warning => "WARNING",
        Severity::Critical => "CRITICAL",
    };
    let head = match f.transition {
        Transition::Firing => format!("🔴 [{sev}] {}", f.rule_id),
        Transition::ReNotify => format!("🔴 [{sev}] (still active) {}", f.rule_id),
        Transition::Resolved => format!("✅ [RESOLVED] {}", f.rule_id),
    };
    format!("{head}\n{}\nVDP: {}", f.message, f.vdp_link)
}

/// Executes f with up to `attempts` retries; exponential backoff of base*2^(n-1) ms between them.
pub fn retry_with_backoff<T, F>(attempts: u32, base_delay_ms: u64, mut f: F) -> anyhow::Result<T>
where
    F: FnMut() -> anyhow::Result<T>,
{
    let mut last_err = None;
    for attempt in 0..attempts.max(1) {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = Some(e);
                if attempt + 1 < attempts {
                    let delay = base_delay_ms.saturating_mul(1u64 << attempt);
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry_with_backoff: no attempts")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::{Severity, Transition};
    use crate::rules::Fired;
    use std::cell::RefCell;

    fn fired() -> Fired {
        Fired {
            rule_id: "sync_stall".into(),
            severity: Severity::Critical,
            transition: Transition::Firing,
            message: "Block height is not increasing".into(),
            vdp_link: "uptime".into(),
        }
    }

    #[test]
    fn format_includes_severity_and_rule() {
        let text = format_message(&fired());
        assert!(text.contains("CRITICAL"));
        assert!(text.contains("sync_stall"));
        assert!(text.contains("uptime"));
    }

    #[test]
    fn resolved_format_marks_resolved() {
        let mut f = fired();
        f.transition = Transition::Resolved;
        let text = format_message(&f);
        assert!(text.to_uppercase().contains("RESOLVED"));
    }

    struct Mock(RefCell<Vec<String>>);
    impl Notifier for Mock {
        fn send(&self, text: &str) -> anyhow::Result<()> {
            self.0.borrow_mut().push(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn mock_notifier_records_send() {
        let m = Mock(RefCell::new(vec![]));
        m.send("hi").unwrap();
        assert_eq!(m.0.borrow().len(), 1);
    }

    #[test]
    fn retry_succeeds_after_transient_failures() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let r: anyhow::Result<u32> = retry_with_backoff(3, 1, || {
            calls.set(calls.get() + 1);
            if calls.get() < 3 { anyhow::bail!("transient") } else { Ok(42) }
        });
        assert_eq!(r.unwrap(), 42);
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn retry_gives_up_after_attempts() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let r: anyhow::Result<u32> = retry_with_backoff(2, 1, || { calls.set(calls.get()+1); anyhow::bail!("always") });
        assert!(r.is_err());
        assert_eq!(calls.get(), 2);
    }
}
