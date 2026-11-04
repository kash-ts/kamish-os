use crate::error::{WebError as AppError, WebResult as AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password using Argon2id with a randomly generated salt.
/// Returns (password_hash, salt_string).
pub fn hash_password(password: &str) -> AppResult<(String, String)> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Argon2 hash failed: {e}")))?
        .to_string();
    Ok((hash, salt.as_str().to_string()))
}

/// Verify a plain text password against a stored PHC formatted Argon2 hash.
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    if let Ok(parsed_hash) = PasswordHash::new(password_hash) {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let (hash, _salt) = hash_password("admin").expect("Hash should succeed");
        assert!(verify_password("admin", &hash));
        assert!(!verify_password("wrong_password", &hash));
    }
}
