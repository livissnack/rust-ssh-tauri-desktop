use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce, Key};
use pbkdf2::pbkdf2;
use sha2::Sha256;
use hmac::Hmac;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use machineid_rs::{IdBuilder, Encryption, HWIDComponent};

// --- 常量定义 ---
const STATIC_SALT: &[u8] = b"l4ILZAwOjGbsLlWbeyyNh1BbgHKhblM2Bcf0IoLYGA";
const PBKDF2_ITERATIONS: u32 = 2000;
const AES_NONCE: &[u8; 12] = b"w5xd0u23v1f1";

// 专门为云同步准备的常量
const SYNC_SALT: &[u8] = b"y3N7t0mYTBf4BxTO0kNAGZlDjGCgUwWfBlGfVVEZ";
const SYNC_ITERATIONS: u32 = 100_000; // 云同步建议使用更高的迭代次数，增加破解难度
const SYNC_NONCE: &[u8; 12] = b"w8w441o8qsjx";

// --- 基础工具函数 ---

fn get_machine_id() -> String {
    let mut builder = IdBuilder::new(Encryption::SHA256);
    builder.add_component(HWIDComponent::SystemID)
           .add_component(HWIDComponent::CPUID);
    builder.build("p1v0lqkzqu9dztvt20da").unwrap_or_else(|_| "fallback-id".into())
}

/// 统一的密钥派生逻辑
fn internal_derive_key(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(
        password.as_bytes(),
        salt,
        iterations,
        &mut key
    ).expect("Key derivation failed");
    key
}

pub fn encrypt_secret(plain_text: &str) -> Result<String, String> {
    if plain_text.is_empty() { return Ok("".into()); }
    let machine_id = get_machine_id();
    let key_bytes = internal_derive_key(&machine_id, STATIC_SALT, PBKDF2_ITERATIONS);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(AES_NONCE);

    let ciphertext = cipher.encrypt(nonce, plain_text.as_bytes())
        .map_err(|e| format!("Encryption error: {}", e))?;
    Ok(STANDARD.encode(ciphertext))
}

pub fn decrypt_secret(encrypted_base64: &str) -> Result<String, String> {
    if encrypted_base64.is_empty() { return Ok("".into()); }
    let machine_id = get_machine_id();
    let key_bytes = internal_derive_key(&machine_id, STATIC_SALT, PBKDF2_ITERATIONS);

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(AES_NONCE);

    let ciphertext = STANDARD.decode(encrypted_base64)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| "Decryption failed (Device mismatch or corrupt data)".to_string())?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF8 error: {}", e))
}

pub fn encrypt_with_key(data: &str, master_key: &str) -> Result<Vec<u8>, String> {
    if master_key.is_empty() { return Err("Master key cannot be empty".into()); }

    let key_bytes = internal_derive_key(master_key, SYNC_SALT, SYNC_ITERATIONS);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(SYNC_NONCE);

    cipher.encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("Sync encryption error: {}", e))
}

pub fn decrypt_with_key(encrypted_data: &[u8], master_key: &str) -> Result<String, String> {
    if master_key.is_empty() { return Err("Master key cannot be empty".into()); }

    let key_bytes = internal_derive_key(master_key, SYNC_SALT, SYNC_ITERATIONS);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(SYNC_NONCE);

    let decrypted = cipher.decrypt(nonce, encrypted_data)
        .map_err(|_| "Sync decryption failed: Incorrect Master Key".to_string())?;

    String::from_utf8(decrypted).map_err(|e| format!("UTF8 error: {}", e))
}