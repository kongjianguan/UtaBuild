//! NetEase Cloud Music EAPI encryption utilities.
//!
//! Implements the EAPI protocol used by the NetEase Cloud Music desktop client:
//! - AES-128-ECB encryption/decryption with key `e82ckenh8dichen8`
//! - MD5 digest for parameter signing
//! - The `encrypt_params` formula that produces the encrypted request body.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;

/// The fixed AES key used by the NetEase EAPI protocol.
const EAPI_KEY: &[u8; 16] = b"e82ckenh8dichen8";

/// Compute a MD5 hex digest string.
pub fn md5_hex(input: &str) -> String {
    format!("{:x}", md5::compute(input.as_bytes()))
}

/// AES-128-ECB encrypt with PKCS7 padding.
fn aes_ecb_encrypt(data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(EAPI_KEY).expect("AES-128 key is 16 bytes");
    let padded = pkcs7_pad(data, 16);
    let mut result = Vec::with_capacity(padded.len());
    for chunk in padded.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    result
}

/// AES-128-ECB decrypt with PKCS7 padding removal.
pub fn aes_ecb_decrypt(data: &[u8]) -> Option<String> {
    if data.is_empty() || data.len() % 16 != 0 {
        return None;
    }
    let cipher = Aes128::new_from_slice(EAPI_KEY).expect("AES-128 key is 16 bytes");
    let mut result = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        result.extend_from_slice(&block);
    }
    // Remove PKCS7 padding
    if let Some(&pad_len) = result.last() {
        let pad_len = pad_len as usize;
        if pad_len > 0 && pad_len <= 16 && result.len() >= pad_len {
            result.truncate(result.len() - pad_len);
        }
    }
    String::from_utf8(result).ok()
}

/// PKCS7 padding.
fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    padded
}

/// Encrypt request parameters using the NetEase EAPI protocol.
///
/// The formula (from lyrico NeCryptoUtils):
/// 1. `message = format!(DIGEST_TEXT, url, json_params)`
/// 2. `digest = md5_hex(message)`
/// 3. `data = format!("{url}-36cd479b6b5-{json_params}-36cd479b6b5-{digest}")`
/// 4. Encrypt `data` with AES-128-ECB → hex-encoded uppercase
pub fn encrypt_params(url: &str, json_params: &str) -> String {
    let message = format!("nobody{}use{}md5forencrypt", url, json_params);
    let digest = md5_hex(&message);
    let data = format!("{url}-36cd479b6b5-{json_params}-36cd479b6b5-{digest}");
    let encrypted = aes_ecb_encrypt(data.as_bytes());
    encrypted.iter().map(|b| format!("{:02X}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hex() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_aes_roundtrip() {
        let plain = "hello world test data";
        let encrypted = aes_ecb_encrypt(plain.as_bytes());
        let decrypted = aes_ecb_decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plain);
    }

    #[test]
    fn test_aes_decrypt_known() {
        // Verify we can decrypt something encrypted with the EAPI key
        let data = b"0123456789abcdef";
        let encrypted = aes_ecb_encrypt(data);
        // PKCS7 padding adds a full block when input is block-aligned
        assert_eq!(encrypted.len(), 32); // 16 data + 16 padding
        let decrypted = aes_ecb_decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.as_bytes(), data);
    }

    #[test]
    fn test_encrypt_params_format() {
        let result = encrypt_params("/api/test", r#"{"key":"value"}"#);
        // Should be a hex string (uppercase)
        assert!(result.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(result, result.to_uppercase());
    }
}
