/// Validate a target version. Accepts `0.14.5` or `v0.14.5`; returns the
/// canonical form WITHOUT a leading `v`. Rejects anything else — this is the
/// guard that keeps untrusted input out of the privileged upgrade command.
pub fn validate(input: &str) -> Option<String> {
    let s = input.strip_prefix('v').unwrap_or(input);
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    for p in &parts {
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    Some(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_and_canonicalizes() {
        assert_eq!(validate("0.14.5").as_deref(), Some("0.14.5"));
        assert_eq!(validate("v0.14.7").as_deref(), Some("0.14.7"));
    }
    #[test]
    fn rejects_malformed_and_injection() {
        for bad in ["0.14", "0.14.5.1", "", "0.14.5; rm -rf /", "0.14.5 && reboot",
                    "../0.14.5", "latest", "v", "0.x.5", " 0.14.5"] {
            assert_eq!(validate(bad), None, "must reject {bad:?}");
        }
    }
}
