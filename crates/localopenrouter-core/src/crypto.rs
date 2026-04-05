use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

use crate::error::{LocalOpenRouterError, Result};

const PBKDF2_ITERATIONS: u32 = 180_000;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const MASTER_CHECK: &[u8] = b"localopenrouter-master-key-v1";
const LEGACY_MASTER_CHECK: &[u8] = b"localrouter-master-key-v1";

pub struct InitializedVault {
    pub key: [u8; KEY_LEN],
    pub salt: Vec<u8>,
    pub check_nonce: Vec<u8>,
    pub check_ciphertext: Vec<u8>,
}

pub fn initialize_master_password(password: &str) -> Result<InitializedVault> {
    let salt = random_bytes(16)?;
    let key = derive_key(password, &salt)?;
    let (check_nonce, check_ciphertext) = encrypt(&key, MASTER_CHECK)?;
    Ok(InitializedVault {
        key,
        salt,
        check_nonce,
        check_ciphertext,
    })
}

pub fn unlock_master_password(
    password: &str,
    salt: &[u8],
    check_nonce: &[u8],
    check_ciphertext: &[u8],
) -> Result<[u8; KEY_LEN]> {
    let key = derive_key(password, salt)?;
    let decrypted = decrypt(&key, check_nonce, check_ciphertext)
        .map_err(|_| LocalOpenRouterError::Crypto("master password is invalid".into()))?;
    if decrypted != MASTER_CHECK && decrypted != LEGACY_MASTER_CHECK {
        return Err(LocalOpenRouterError::Crypto(
            "master password is invalid".into(),
        ));
    }
    Ok(key)
}

pub fn encrypt_secret(key: &[u8; KEY_LEN], plaintext: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    encrypt(key, plaintext.as_bytes())
}

pub fn decrypt_secret(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Result<String> {
    let plaintext = decrypt(key, nonce, ciphertext)?;
    String::from_utf8(plaintext)
        .map_err(|_| LocalOpenRouterError::Crypto("secret is not valid utf-8".into()))
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let iterations = NonZeroU32::new(PBKDF2_ITERATIONS)
        .ok_or_else(|| LocalOpenRouterError::Crypto("invalid PBKDF2 iteration count".into()))?;
    let mut output = [0_u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iterations,
        salt,
        password.as_bytes(),
        &mut output,
    );
    Ok(output)
}

fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut nonce_bytes = random_bytes(NONCE_LEN)?;
    let mut in_out = plaintext.to_vec();
    let key = LessSafeKey::new(
        UnboundKey::new(&CHACHA20_POLY1305, key)
            .map_err(|_| LocalOpenRouterError::Crypto("failed to initialize cipher".into()))?,
    );
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(
            nonce_bytes
                .clone()
                .try_into()
                .map_err(|_| LocalOpenRouterError::Crypto("invalid nonce length".into()))?,
        ),
        Aad::empty(),
        &mut in_out,
    )
    .map_err(|_| LocalOpenRouterError::Crypto("failed to encrypt secret".into()))?;
    Ok((std::mem::take(&mut nonce_bytes), in_out))
}

fn decrypt(key: &[u8; KEY_LEN], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let key = LessSafeKey::new(
        UnboundKey::new(&CHACHA20_POLY1305, key)
            .map_err(|_| LocalOpenRouterError::Crypto("failed to initialize cipher".into()))?,
    );
    let mut in_out = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(
            Nonce::assume_unique_for_key(
                nonce
                    .try_into()
                    .map_err(|_| LocalOpenRouterError::Crypto("invalid nonce length".into()))?,
            ),
            Aad::empty(),
            &mut in_out,
        )
        .map_err(|_| LocalOpenRouterError::Crypto("failed to decrypt secret".into()))?;
    Ok(plaintext.to_vec())
}

fn random_bytes(len: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0_u8; len];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| LocalOpenRouterError::Crypto("failed to read secure randomness".into()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        decrypt_secret, encrypt_secret, initialize_master_password, unlock_master_password,
    };

    #[test]
    fn master_password_round_trip() {
        let material = initialize_master_password("correct horse battery staple").unwrap();
        let unlocked = unlock_master_password(
            "correct horse battery staple",
            &material.salt,
            &material.check_nonce,
            &material.check_ciphertext,
        )
        .unwrap();
        let (nonce, ciphertext) = encrypt_secret(&unlocked, "sk-localopenrouter").unwrap();
        let decrypted = decrypt_secret(&unlocked, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, "sk-localopenrouter");
    }
}
