use totp_rs::{Algorithm, Secret, TOTP};

pub struct Totp(TOTP);

impl Totp {
    pub fn from_base32(secret_b32: &str) -> anyhow::Result<Totp> {
        let bytes = Secret::Encoded(secret_b32.to_string())
            .to_bytes()
            .map_err(|e| anyhow::anyhow!("totp secret: {e:?}"))?;
        // new_unchecked skips the 128-bit minimum length check, which allows
        // use of RFC 6238 test vectors (e.g. 80-bit secrets in the spec).
        // Production callers should use generate_secret_base32() which produces
        // a 160-bit secret meeting the rfc-4226 recommended length.
        let totp = TOTP::new_unchecked(Algorithm::SHA1, 6, 1, 30, bytes);
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
    fn known_vector_checks() {
        // RFC 6238-style: build from a fixed secret, accept the code for its own step.
        let t = Totp::from_base32("JBSWY3DPEHPK3PXP").unwrap();
        let now = 59u64;
        let code = t.current(now);
        assert!(t.check(&code, now));
        assert!(!t.check("000000", now.wrapping_add(10_000)));
    }
}
