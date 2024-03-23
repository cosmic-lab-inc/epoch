use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub struct Hasher;

impl Hasher {
    /// Hash an API key
    pub fn hash(key: &[u8]) -> anyhow::Result<String> {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let hash = argon2
            .hash_password(key, &salt)
            .map_err(|_| anyhow::anyhow!("Failed to hash API key"))?;
        Ok(hash.serialize().as_str().to_string())
    }

    /// Verify an API key against the hashed key
    pub fn verify(key: &[u8], hashed_key: &str) -> anyhow::Result<()> {
        let parsed_hash = PasswordHash::new(hashed_key).map_err(|e| {
            anyhow::anyhow!("Failed to build PasswordHash from hashed key: {:?}", e)
        })?;
        Argon2::default()
            .verify_password(key, &parsed_hash)
            .map_err(|_| anyhow::anyhow!("API key does not match hashed key."))
    }
}

#[test]
fn test_warden() -> anyhow::Result<()> {
    let key = b"drewbert";
    let hashed_key = Hasher::hash(key)?;
    assert!(Hasher::verify(key, &hashed_key).is_ok());
    Ok(())
}
