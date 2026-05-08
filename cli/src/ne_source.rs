//! NetEase Cloud Music (网易云音乐) lyric source.
//!
//! Uses the NetEase EAPI protocol (encrypted) to:
//! 1. Perform anonymous login to obtain session cookies
//! 2. Search for songs by keyword
//! 3. Fetch lyrics (LRC + YRC + translated + romanized)
//!
//! Reference: lyrico NeSource.kt

use crate::models::{LyricElement, SearchResult};
use crate::ne_crypto;
use crate::lrc_parser;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// NetEase Cloud Music searcher.
pub struct NeteaseSource {
    client: Client,
    /// Cached cookies from anonymous login (MUSIC_A, NMTID, __csrf, etc.)
    cookies: Arc<Mutex<Option<NeteaseSession>>>,
}

#[derive(Debug, Clone)]
struct NeteaseSession {
    cookies: Vec<(String, String)>,
    #[allow(dead_code)]
    user_id: u64,
}

/// NetEase EAPI base URL.
const NE_API_BASE: &str = "https://interface.music.163.com";

/// App version string (mimics desktop client).
const APP_VER: &str = "3.1.3.203419";

impl NeteaseSource {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/3.1.3.203419")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::ACCEPT,
                    "*/*".parse().unwrap(),
                );
                headers.insert(
                    reqwest::header::ACCEPT_LANGUAGE,
                    "zh-CN,zh;q=0.9".parse().unwrap(),
                );
                headers
            })
            .no_proxy()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            cookies: Arc::new(Mutex::new(None)),
        }
    }

    /// Ensure we have a valid anonymous session.
    async fn ensure_session(&self) -> Option<NeteaseSession> {
        {
            let guard = self.cookies.lock().await;
            if let Some(ref session) = *guard {
                return Some(session.clone());
            }
        }

        let session = self.anonymous_login().await?;
        let mut guard = self.cookies.lock().await;
        *guard = Some(session.clone());
        Some(session)
    }

    /// Perform anonymous login to obtain MUSIC_A / NMTID / __csrf cookies.
    async fn anonymous_login(&self) -> Option<NeteaseSession> {
        let device_id = uuid_v4();
        let client_sign = generate_client_sign();

        let path = "/eapi/register/anonimous";
        let username = anonimous_username(&device_id);

        let params = serde_json::json!({
            "username": username,
            "e_r": true,
        });

        let pre_cookies = vec![
            ("os", "pc"),
            ("deviceId", device_id.as_str()),
            ("osver", "Microsoft-Windows-10--build-22621-64bit"),
            ("clientSign", &client_sign),
            ("channel", "netease"),
            ("mode", "ASRock X670E Taichi"),
            ("appver", APP_VER),
        ];

        let cookie_str: String = pre_cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        let body = build_eapi_body(path, &params, &pre_cookies);

        let resp = self
            .client
            .post(format!("{}{}", NE_API_BASE, path))
            .header("Referer", "https://music.163.com/")
            .header("Cookie", &cookie_str)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .ok()?;

        // Save headers BEFORE consuming the body
        let set_cookie_headers: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(String::from))
            .collect();

        let resp_bytes = resp.bytes().await.ok()?;
        let decrypted = ne_crypto::aes_ecb_decrypt(&resp_bytes)?;
        debug!("NetEase login response: {}", &decrypted[..decrypted.len().min(200)]);

        let json: serde_json::Value = serde_json::from_str(&decrypted).ok()?;

        // Handle both integer (200) and string ("200") response codes
        let code_ok = json
            .get("code")
            .map(|v| v.as_i64() == Some(200) || v.as_str() == Some("200"))
            .unwrap_or(false);
        if !code_ok {
            warn!("NetEase login failed: {:?}", json.get("message"));
            return None;
        }

        let user_id = json.get("userId")?.as_i64()? as u64;

        // Parse real session cookie values from Set-Cookie response headers
        let mut response_cookies: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for cookie_line in &set_cookie_headers {
            if let Some(cookie_pair) = cookie_line.split(';').next() {
                if let Some(eq_pos) = cookie_pair.find('=') {
                    let key = cookie_pair[..eq_pos].to_string();
                    let value = cookie_pair[eq_pos + 1..].to_string();
                    response_cookies.insert(key, value);
                }
            }
        }

        let mut cookies: Vec<(String, String)> = pre_cookies
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        // Use real session values from server, fall back to empty strings
        for key in &["MUSIC_A", "NMTID", "__csrf"] {
            let value = response_cookies
                .get(*key)
                .cloned()
                .unwrap_or_default();
            cookies.push((key.to_string(), value));
        }

        Some(NeteaseSession { cookies, user_id })
    }

    /// Send an EAPI request and return the decrypted JSON response.
    async fn eapi_request(
        &self,
        session: &NeteaseSession,
        path: &str,
        params: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let cookie_pairs: Vec<(&str, &str)> = session
            .cookies
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let body = build_eapi_body(path, params, &cookie_pairs);

        let cookie_str: String = session
            .cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");

        let resp = self
            .client
            .post(format!("{}{}", NE_API_BASE, path))
            .header("Referer", "https://music.163.com/")
            .header("Cookie", &cookie_str)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .ok()?;

        let resp_bytes = resp.bytes().await.ok()?;
        let decrypted = ne_crypto::aes_ecb_decrypt(&resp_bytes)?;
        serde_json::from_str(&decrypted).ok()
    }

    /// Search for songs by keyword.
    pub async fn search(
        &self,
        keyword: &str,
        page: u32,
        page_size: u32,
    ) -> Option<Vec<SearchResult>> {
        let session = self.ensure_session().await?;
        let path = "/eapi/search/song/list/page";
        let offset = (page.saturating_sub(1)) * page_size;

        let params = serde_json::json!({
            "limit": page_size.to_string(),
            "offset": offset.to_string(),
            "keyword": keyword,
            "scene": "NORMAL",
            "needCorrect": "true",
        });

        let json = self.eapi_request(&session, path, &params).await?;
        debug!("NetEase search response code: {:?}", json.get("code"));

        if json.get("code")?.as_i64()? != 200 {
            return None;
        }

        let resources = json
            .pointer("/data/resources")?
            .as_array()?;

        let results: Vec<SearchResult> = resources
            .iter()
            .filter_map(|res| {
                let song = res.pointer("/baseInfo/simpleSongData")?;
                let id = song.get("id")?.as_i64()?;
                let name = song.get("name")?.as_str()?.to_string();
                // NetEase EAPI uses short field names: "ar" for artists, "al" for album
                let artists: Vec<String> = song
                    .get("ar")?
                    .as_array()?
                    .iter()
                    .filter_map(|a| a.get("name")?.as_str().map(String::from))
                    .collect();
                let artist = artists.join("/");
                let album = song
                    .pointer("/al/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(SearchResult {
                    title: name,
                    artist,
                    url: format!("ne:{}", id),
                    matched: false,
                    lyricist: None,
                    composer: None,
                    album: Some(album),
                    cover_url: None,
                    source: "netease".to_string(),
                })
            })
            .collect();

        Some(results)
    }

    /// Fetch lyrics for a song by its NetEase song ID.
    /// Returns `LyricElement`s with ruby annotations when romaji data
    /// (`romalrc` field) is available from the NetEase API.
    pub async fn fetch_lyrics(&self, song_id: &str) -> Option<Vec<LyricElement>> {
        let session = self.ensure_session().await?;
        let path = "/eapi/song/lyric/v1";
        let id: i64 = song_id.parse().ok()?;

        let params = serde_json::json!({
            "id": id,
            "lv": "-1",
            "tv": "-1",
            "rv": "-1",
            "yv": "-1",
        });

        let json = self.eapi_request(&session, path, &params).await?;
        debug!("NetEase lyric response code: {:?}", json.get("code"));

        let yrc_lyric = json
            .pointer("/yrc/lyric")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let lrc_lyric = json
            .pointer("/lrc/lyric")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        let romalrc = json
            .pointer("/romalrc/lyric")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());

        let elements = lrc_parser::parse_lyrics_with_ruby(
            yrc_lyric,
            lrc_lyric,
            romalrc,
        );

        if elements.is_empty() {
            None
        } else {
            Some(crate::ruby_align::sanitize_ruby_elements(elements))
        }
    }
}

impl Default for NeteaseSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the EAPI request body (URL-encoded `params=<hex>`).
fn build_eapi_body(
    path: &str,
    params: &serde_json::Value,
    cookies: &[(&str, &str)],
) -> String {
    // Build header param from cookies
    let header = serde_json::json!({
        "clientSign": cookies.iter().find(|(k,_)| *k == "clientSign").map(|(_,v)| *v).unwrap_or(""),
        "osver": cookies.iter().find(|(k,_)| *k == "osver").map(|(_,v)| *v).unwrap_or(""),
        "deviceId": cookies.iter().find(|(k,_)| *k == "deviceId").map(|(_,v)| *v).unwrap_or(""),
        "os": cookies.iter().find(|(k,_)| *k == "os").map(|(_,v)| *v).unwrap_or("pc"),
        "appver": cookies.iter().find(|(k,_)| *k == "appver").map(|(_,v)| *v).unwrap_or(APP_VER),
        "requestId": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string(),
    });

    let mut params_map = match params.clone() {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    params_map.insert("header".to_string(), serde_json::json!(header.to_string()));
    params_map.entry("e_r".to_string()).or_insert(serde_json::json!(true));

    let params_str = serde_json::to_string(&params_map).unwrap_or_default();
    let encrypt_path = path.replace("/eapi/", "/api/");
    let encrypted_hex = ne_crypto::encrypt_params(&encrypt_path, &params_str);
    format!("params={}", encrypted_hex)
}

/// Generate a random device ID (UUID v4 without dashes).
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let rand_a: u64 = (now.as_nanos() & 0xFFFF_FFFF_FFFF) as u64;
    let rand_b: u64 = (now.as_nanos() >> 16) as u64;
    format!(
        "{:08x}{:04x}4{:03x}{:04x}{:012x}",
        rand_a as u32,
        (rand_a >> 32) as u16 & 0x0FFF,
        (rand_b as u16) & 0x0FFF,
        (rand_b >> 16) as u16 & 0x3FFF | 0x8000,
        rand_b >> 32
    )
}

/// Generate the clientSign string.
///
/// Format: `MAC@@@RANDOM@@@@@@HASH` where:
/// - MAC = 6 colon-separated uppercase hex bytes
/// - RANDOM = 8 uppercase ASCII letters
/// - HASH = 64 lowercase hex chars
/// Matches Lyrico's NeSource.generateClientSign().
fn generate_client_sign() -> String {
    let mac: String = (0..6)
        .map(|_| format!("{:02X}", fast_rand_byte()))
        .collect::<Vec<_>>()
        .join(":");
    // Use only uppercase A-Z for the random string (matching Lyrico)
    let random_str: String = (0..8)
        .map(|_| {
            let idx = (fast_rand_byte() as usize) % 26;
            (b'A' + idx as u8) as char
        })
        .collect();
    // Lowercase hex characters (0-9, a-f)
    let hex_chars = b"0123456789abcdef";
    let hash_part: String = (0..64)
        .map(|_| {
            let idx = (fast_rand_byte() as usize) % 16;
            hex_chars[idx] as char
        })
        .collect();
    format!("{}@@@{}@@@@@@{}", mac, random_str, hash_part)
}

/// Fast pseudo-random byte (non-crypto).
fn fast_rand_byte() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (ns as u8).wrapping_add((ns >> 8) as u8)
}

/// Generate the anonimous username (XOR with key + MD5 + base64).
///
/// Matches Lyrico's NeSource.getAnonimousUsername():
///   1. XOR device_id with key
///   2. MD5 the XORed bytes, Base64-encode the digest (NOT hex!)
///   3. Combine: `"$deviceId $base64Md5"`
///   4. Base64-encode the combined string
fn anonimous_username(device_id: &str) -> String {
    let key = "3go8&$8*3*3h0k(2)2";
    let xored: String = device_id
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let key_char = key.chars().nth(i % key.len()).unwrap_or('0');
            (c as u8 ^ key_char as u8) as char
        })
        .collect();
    use base64::Engine;
    // MD5 digest → Base64 (NOT hex! Lyrico uses Base64)
    let md5_digest = md5::compute(xored.as_bytes());
    let base64_md5 = base64::engine::general_purpose::STANDARD.encode(md5_digest.as_ref());
    let combined = format!("{} {}", device_id, base64_md5);
    base64::engine::general_purpose::STANDARD.encode(combined.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_eapi_body() {
        let path = "/eapi/search/song/list/page";
        let params = serde_json::json!({"keyword": "test"});
        let cookies: Vec<(&str, &str)> = vec![
            ("os", "pc"),
            ("appver", APP_VER),
            ("deviceId", "abc123"),
            ("clientSign", "00:00:00:00:00:00@@@ABCDEFGH@@@@@@aaaa"),
            ("osver", "Windows-10"),
        ];
        let body = build_eapi_body(path, &params, &cookies);
        assert!(body.starts_with("params="));
        assert!(body.len() > 7);
    }

    #[test]
    fn test_anonimous_username() {
        let name = anonimous_username("abc123");
        // Should be valid base64
        use base64::Engine;
        assert!(base64::engine::general_purpose::STANDARD.decode(&name).is_ok());
    }
}
