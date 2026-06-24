pub trait ServiceProbe: Send + Sync {
    fn is_active(&self, unit: &str) -> bool;
}

pub trait VersionProbe: Send + Sync {
    fn version(&self, binary: &str) -> Option<String>;
}

pub trait LogReader: Send + Sync {
    fn tail(&self, unit: &str, lines: usize) -> Vec<String>;
}

pub struct SystemctlProbe;
impl ServiceProbe for SystemctlProbe {
    fn is_active(&self, unit: &str) -> bool {
        std::process::Command::new("systemctl")
            .arg("is-active")
            .arg(unit)
            .output()
            .map(|o| o.stdout.starts_with(b"active"))
            .unwrap_or(false)
    }
}

pub struct BinaryVersionProbe;
impl VersionProbe for BinaryVersionProbe {
    fn version(&self, binary: &str) -> Option<String> {
        let out = std::process::Command::new(binary).arg("--version").output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let brace = text.find('{')?;
        let json: serde_json::Value = serde_json::from_str(text[brace..].trim()).ok()?;
        json.get("tag")?.as_str().map(|s| s.to_string())
    }
}

pub struct JournalReader;
impl LogReader for JournalReader {
    fn tail(&self, unit: &str, lines: usize) -> Vec<String> {
        let out = std::process::Command::new("journalctl")
            .args(["-u", unit, "-n", &lines.to_string(), "--no-pager", "-q"])
            .output();
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect(),
            Err(_) => vec![],
        }
    }
}
