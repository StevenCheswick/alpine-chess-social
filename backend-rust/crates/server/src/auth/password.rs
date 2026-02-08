use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

/// Hash a password with argon2id (OWASP recommended).
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

/// Verify a password against a hash.
/// Supports both argon2 (new) and bcrypt (migrated) hashes.
/// Returns (is_valid, needs_rehash).
pub fn verify_password(password: &str, hash: &str) -> Result<(bool, bool), String> {
    if hash.starts_with("$argon2") {
        // Argon2 hash
        let parsed = PasswordHash::new(hash).map_err(|e| e.to_string())?;
        let valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        Ok((valid, false))
    } else if hash.starts_with("$2b$") || hash.starts_with("$2a$") {
        // Legacy bcrypt hash — verify then signal rehash needed
        let valid = bcrypt::verify(password, hash).unwrap_or(false);
        Ok((valid, valid)) // needs_rehash only if password was correct
    } else {
        Err("Unknown hash format".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2"));

        let (valid, needs_rehash) = verify_password(password, &hash).unwrap();
        assert!(valid);
        assert!(!needs_rehash);

        let (valid, _) = verify_password("wrong_password", &hash).unwrap();
        assert!(!valid);
    }
}
