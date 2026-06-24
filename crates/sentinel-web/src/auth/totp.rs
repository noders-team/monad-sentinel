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

#[cfg(test)]
mod tests {
    use super::*;

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
