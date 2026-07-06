use totp_rs::{Algorithm, Secret, TOTP};

pub struct Totp(TOTP);

impl Totp {
    pub fn from_base32(secret_b32: &str) -> anyhow::Result<Totp> {
        let bytes = Secret::Encoded(secret_b32.to_string())
            .to_bytes()
            .map_err(|e| anyhow::anyhow!("totp secret: {e:?}"))?;
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes)
            .map_err(|e| anyhow::anyhow!("totp config: {e:?}"))?;
        Ok(Totp(totp))
    }

    pub fn current(&self, unix_secs: u64) -> String {
        self.0.generate(unix_secs)
    }

    pub fn check(&self, code: &str, unix_secs: u64) -> bool {
        self.0.check(code, unix_secs)
    }
}

/// Encode 20 raw bytes as a base32 string suitable for use as a TOTP secret.
pub fn generate_secret_base32(rng_bytes: [u8; 20]) -> String {
    Secret::Raw(rng_bytes.to_vec()).to_encoded().to_string()
}

/// The otpauth:// URI the operator scans once during enrollment.
pub fn enrollment_uri(secret_b32: &str) -> String {
    format!(
        "otpauth://totp/SentinelConsole:admin?secret={secret_b32}&issuer=SentinelConsole&algorithm=SHA1&digits=6&period=30"
    )
}

/// Write the one-time enrollment banner to `path`, readable by the owner only.
/// Keeps the TOTP secret out of stderr/journald, where it would be retained
/// indefinitely and readable by anyone with journal access.
pub fn write_enrollment_file(path: &std::path::Path, secret_b32: &str) -> anyhow::Result<()> {
    use std::io::Write;

    let contents = format!(
        "=== TOTP ENROLLMENT — scan ONCE with your authenticator app, then DELETE this file ===\n\
         Secret (base32): {secret_b32}\n\
         OTPAuth URI    : {}\n",
        enrollment_uri(secret_b32)
    );

    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts
        .open(path)
        .map_err(|e| anyhow::anyhow!("cannot write enrollment file {}: {e}", path.display()))?;
    // create(true) keeps the mode of a pre-existing file — pin it explicitly.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(contents.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_file_is_owner_only_and_contains_uri() {
        let dir = std::env::temp_dir().join(format!("sentinel-enroll-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("totp-enroll.txt");

        let secret = generate_secret_base32([9u8; 20]);
        write_enrollment_file(&path, &secret).expect("write enrollment file");

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(&secret));
        assert!(text.contains("otpauth://totp/SentinelConsole:admin"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "enrollment file must be readable by owner only");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn checked_constructor_rejects_short_secret() {
        // An 80-bit (10-byte) secret must be rejected by the checked constructor.
        let short_b32 = Secret::Raw(vec![0u8; 10]).to_encoded().to_string();
        assert!(Totp::from_base32(&short_b32).is_err());
    }

    #[test]
    fn known_vector_checks() {
        // Use a 160-bit (20-byte) secret produced by generate_secret_base32,
        // which the checked constructor accepts.
        let fixed_bytes: [u8; 20] = [
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a,
            0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54,
        ];
        let secret = generate_secret_base32(fixed_bytes);
        let t = Totp::from_base32(&secret).unwrap();

        let now = 59u64;
        let code = t.current(now);
        // The correct code at 'now' must pass.
        assert!(t.check(&code, now));
        // A code from a far-apart time step must be rejected at 'now'.
        let far_ts = now + 300; // 10 steps away, well outside the 1-step window
        let far_code = t.current(far_ts);
        assert!(!t.check(&far_code, now));
    }
}
