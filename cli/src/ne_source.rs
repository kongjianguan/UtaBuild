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
/// 网易云音乐搜索器，提供匿名登录、歌曲搜索、歌词获取等功能。
pub struct NeteaseSource {
    /// HTTP 客户端，用于发送网络请求。
    client: Client,
    /// Cached cookies from anonymous login (MUSIC_A, NMTID, __csrf, etc.)
    /// 缓存匿名登录后获取的会话 Cookie，包含 MUSIC_A、NMTID、__csrf 等。
    cookies: Arc<Mutex<Option<NeteaseSession>>>,
}

/// 网易云音乐匿名会话信息。
#[derive(Debug, Clone)]
struct NeteaseSession {
    /// Cookie 键值对列表，包含预置参数和服务器返回的会话凭证。
    cookies: Vec<(String, String)>,
    #[allow(dead_code)]
    /// 匿名登录后获取的用户 ID。
    user_id: u64,
}

/// NetEase EAPI base URL.
/// 网易云音乐 EAPI 接口基础地址。
const NE_API_BASE: &str = "https://interface.music.163.com";

/// App version string (mimics desktop client).
/// 应用版本号，模拟桌面客户端标识。
const APP_VER: &str = "3.1.3.203419";

impl NeteaseSource {
    /// 创建一个新的 NeteaseSource 实例。
    ///
    /// 初始化 HTTP 客户端，配置模拟浏览器 User-Agent、请求头、代理和超时等参数。
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
    /// 确保存在有效的匿名会话，如果缓存中没有则执行匿名登录。
    ///
    /// 优先返回已缓存的会话；若尚未登录则自动调用 `anonymous_login` 并缓存结果。
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
    /// 执行匿名登录，获取 MUSIC_A、NMTID、__csrf 等会话 Cookie。
    ///
    /// 生成随机设备 ID 和客户端签名，构造匿名用户名，向 `/eapi/register/anonimous` 接口发送请求，
    /// 解析返回的 Set-Cookie 头和加密响应体，构建完整的会话信息。
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

        if !resp.status().is_success() {
            warn!("NetEase login returned HTTP {}", resp.status());
            return None;
        }

        // Save headers BEFORE consuming the body
        // 在消费响应体之前保存 Set-Cookie 响应头
        let set_cookie_headers: Vec<String> = resp
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(String::from))
            .collect();

        let resp_bytes = resp.bytes().await.ok()?;
        let decrypted = ne_crypto::aes_ecb_decrypt(&resp_bytes)?;
        // Slice on a char boundary — byte slicing can panic mid multi-byte char.
        let preview: String = decrypted.chars().take(200).collect();
        debug!("NetEase login response: {}", preview);

        let json: serde_json::Value = serde_json::from_str(&decrypted).ok()?;

        // Handle both integer (200) and string ("200") response codes
        // 处理返回码，兼容整数型（200）和字符串型（"200"）两种格式
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
        // 从 Set-Cookie 响应头中解析真实的会话 Cookie 值
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
        // 使用服务器返回的会话值，缺失时回退为空字符串
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
    /// 发送 EAPI 请求，返回解密后的 JSON 响应。
    ///
    /// 基于会话中的 Cookie 构建请求，对加密的响应体进行 AES ECB 解密后解析为 JSON。
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

        if !resp.status().is_success() {
            warn!("NetEase EAPI request returned HTTP {}", resp.status());
            return None;
        }

        let resp_bytes = resp.bytes().await.ok()?;
        let decrypted = ne_crypto::aes_ecb_decrypt(&resp_bytes)?;
        serde_json::from_str(&decrypted).ok()
    }

    /// Search for songs by keyword.
    /// 根据关键词搜索歌曲。
    ///
    /// 向 `/eapi/search/song/list/page` 接口发起搜索请求，解析返回的歌曲列表，
    /// 提取歌曲 ID、名称、艺术家和专辑信息，转换为 `SearchResult` 列表。
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
                // 网易 EAPI 使用缩写字段名："ar" 表示艺术家列表，"al" 表示专辑信息
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
    /// 根据网易云音乐歌曲 ID 获取歌词。
    ///
    /// 从 API 获取 YRC（逐字歌词）、LRC（标准歌词）和 romalrc（罗马音）三种歌词数据，
    /// 通过 `lrc_parser::parse_lyrics_with_ruby` 合并解析，再经 `ruby_align::sanitize_ruby_elements`
    /// 清洗后返回。若存在罗马音数据，则会生成包含注音（ruby）标注的歌词元素。
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
    /// 提供默认构造，等价于 `NeteaseSource::new()`。
    fn default() -> Self {
        Self::new()
    }
}

/// Build the EAPI request body (URL-encoded `params=<hex>`).
/// 构建 EAPI 请求体，格式为 URL 编码的 `params=<hex>`。
///
/// 从 Cookie 中提取客户端签名、系统版本、设备 ID、操作系统和应用版本等参数，
/// 构造 header JSON 并合并到请求参数中，然后使用 `ne_crypto::encrypt_params`
/// 对拼接后的参数进行加密，最终返回 `params=<加密后的十六进制字符串>` 格式的请求体。
fn build_eapi_body(
    path: &str,
    params: &serde_json::Value,
    cookies: &[(&str, &str)],
) -> String {
    // Build header param from cookies
    // 从 Cookie 中提取并构造 header 参数
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
/// 生成随机设备 ID，格式为不带连字符的 UUID v4。
///
/// 基于系统时间戳生成伪随机数，构造符合 UUID v4 格式的 32 位十六进制字符串。
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
/// 生成客户端签名（clientSign）字符串。
///
/// Format: `MAC@@@RANDOM@@@@@@HASH` where:
/// - MAC = 6 colon-separated uppercase hex bytes
/// - RANDOM = 8 uppercase ASCII letters
/// - HASH = 64 lowercase hex chars
/// Matches Lyrico's NeSource.generateClientSign().
/// 格式：`MAC@@@RANDOM@@@@@@HASH`
/// - MAC：6 个冒号分隔的大写十六进制字节
/// - RANDOM：8 个大写 ASCII 字母
/// - HASH：64 个小写十六进制字符
/// 与 Lyrico 的 NeSource.generateClientSign() 保持一致。
fn generate_client_sign() -> String {
    let mac: String = (0..6)
        .map(|_| format!("{:02X}", fast_rand_byte()))
        .collect::<Vec<_>>()
        .join(":");
    // Use only uppercase A-Z for the random string (matching Lyrico)
    // 随机字符串仅使用大写字母 A-Z，与 Lyrico 保持一致
    let random_str: String = (0..8)
        .map(|_| {
            let idx = (fast_rand_byte() as usize) % 26;
            (b'A' + idx as u8) as char
        })
        .collect();
    // Lowercase hex characters (0-9, a-f)
    // 小写十六进制字符（0-9, a-f）
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
/// 快速伪随机字节生成器（非密码学安全）。
///
/// 基于系统时间戳的纳秒值进行运算，返回一个伪随机字节。
fn fast_rand_byte() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (ns as u8).wrapping_add((ns >> 8) as u8)
}

/// Generate the anonimous username (XOR with key + MD5 + base64).
/// 生成匿名登录用户名。
///
/// Matches Lyrico's NeSource.getAnonimousUsername():
///   1. XOR device_id with key
///   2. MD5 the XORed bytes, Base64-encode the digest (NOT hex!)
///   3. Combine: `"$deviceId $base64Md5"`
///   4. Base64-encode the combined string
/// 算法步骤与 Lyrico 的 NeSource.getAnonimousUsername() 一致：
///   1. 将 device_id 与密钥逐字节异或
///   2. 对异或结果进行 MD5 哈希，对摘要进行 Base64 编码（注意不是十六进制编码）
///   3. 拼接：`"$deviceId $base64Md5"`
///   4. 对拼接后的字符串进行 Base64 编码
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
    // MD5 摘要 → Base64 编码（注意不是十六进制，Lyrico 使用 Base64）
    let md5_digest = md5::compute(xored.as_bytes());
    let base64_md5 = base64::engine::general_purpose::STANDARD.encode(md5_digest.as_ref());
    let combined = format!("{} {}", device_id, base64_md5);
    base64::engine::general_purpose::STANDARD.encode(combined.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 `build_eapi_body` 函数是否能正确生成 EAPI 请求体。
    ///
    /// 验证返回的字符串以 "params=" 开头且长度大于 7（即至少包含加密内容）。
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

    /// 测试 `anonimous_username` 函数生成的用户名是否为合法的 Base64 字符串。
    #[test]
    fn test_anonimous_username() {
        let name = anonimous_username("abc123");
        // Should be valid base64
        // 验证输出是合法的 Base64 编码
        use base64::Engine;
        assert!(base64::engine::general_purpose::STANDARD.decode(&name).is_ok());
    }
}
