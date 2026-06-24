use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn hash(pw: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let phc = Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash: {e}"))?
        .to_string();
    Ok(phc)
}

pub fn verify(pw: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_then_verify_roundtrip() {
        let phc = hash("correct horse").unwrap();
        assert!(verify("correct horse", &phc));
        assert!(!verify("wrong", &phc));
    }
    #[test]
    fn verify_bad_phc_is_false_not_panic() {
        assert!(!verify("x", "not-a-phc-string"));
    }
}
