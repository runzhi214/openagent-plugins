#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
type HmacSha256 = Hmac<Sha256>;

use openagent_pdk::prelude::serde_json;
use serde::Deserialize;

pub const DEFAULT_DOMAIN: &str = "devstation.myhuaweicloud.com";
pub const API_PATH: &str = "/open-api-public/v1/tokenhub-configs";
pub const API_PATH_SIGN: &str = "/open-api-public/v1/tokenhub-configs/";
pub const PROVIDER: &str = "hwdevspace";
const HEX: &[u8; 16] = b"0123456789abcdef";

#[derive(Deserialize)]
pub struct ModelInfo {
    pub model_id: String,
    #[allow(dead_code)]
    pub model_name: String,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub max_tokens: u64,
}

#[derive(Deserialize)]
pub struct AgentConfig {
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ApiResponse {
    pub error_code: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub error_msg: String,
    pub result: AgentConfig,
}

pub fn get_models(ak: &str, sk: &str, security_token: Option<&str>) -> Option<AgentConfig> {
    let ts = utc_nanos_to_timestamp(openagent_pdk::host::utc_now());

    let domain = DEFAULT_DOMAIN;
    let url = alloc::format!("https://{}{}", domain, API_PATH);

    let auth = sign_request(ak, sk, domain, API_PATH_SIGN, &ts);
    let headers = build_headers(&auth, domain, &ts, security_token);

    openagent_pdk::host::log_info("tokenhub: calling API");
    let url_msg = alloc::format!("tokenhub: url={}", url);
    openagent_pdk::host::log_info(&url_msg);

    let (status, body) = match openagent_pdk::host::http_request("GET", &url, &headers, &[]) {
        Ok(r) => r,
        Err(e) => {
            let warn = alloc::format!("tokenhub: http error={}", e);
            openagent_pdk::host::log_warn(&warn);
            return None;
        }
    };

    if status != 200 {
        let warn = alloc::format!("tokenhub: API status={}", status);
        openagent_pdk::host::log_warn(&warn);
        let body_msg = alloc::format!("tokenhub: response body={}", body);
        openagent_pdk::host::log_warn(&body_msg);
        return None;
    }

    let resp: ApiResponse = serde_json::from_str(&body).ok()?;
    if resp.error_code != "0000" {
        openagent_pdk::host::log_warn("tokenhub: API error code not 0000");
        return None;
    }
    let mut result = resp.result;
    override_limits(&mut result.models);
    Some(result)
}

fn override_limits(models: &mut [ModelInfo]) {
    for m in models.iter_mut() {
        let (cw, mt) = match m.model_id.as_str() {
            "glm-5.2" => (131072, 131072),
            "glm-5.1" => (131072, 131072),
            "glm-5" => (131072, 65536),
            "openpangu-2.0-flash" => (512000, 131072),
            "deepseek-r1-250528" => (98304, 32768),
            "deepseek-v3.2" => (131072, 32768),
            "DeepSeek-V3" => (131072, 32768),
            "deepseek-v3.1-terminus" => (131072, 32768),
            _ => continue,
        };
        m.context_window = cw;
        m.max_tokens = mt;
    }
}

fn sign_request(ak: &str, sk: &str, host: &str, path: &str, timestamp: &str) -> String {
    let method = "GET";
    let query = "";
    let payload = hex(&sha256(b""));

    let signed_headers = "host;x-sdk-date";
    let canonical_headers = alloc::format!("host:{}\nx-sdk-date:{}\n", host, timestamp);
    let canonical_request = alloc::format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method,
        path,
        query,
        canonical_headers,
        signed_headers,
        payload
    );

    let hashed = hex(&sha256(canonical_request.as_bytes()));

    let string_to_sign = alloc::format!("SDK-HMAC-SHA256\n{}\n{}", timestamp, hashed);

    let signature = hex(&hmac_sha256_owned(sk.as_bytes(), string_to_sign.as_bytes()));

    alloc::format!(
        "SDK-HMAC-SHA256 Access={}, SignedHeaders={}, Signature={}",
        ak,
        signed_headers,
        signature
    )
}

fn build_headers(
    auth: &str,
    host_val: &str,
    timestamp: &str,
    security_token: Option<&str>,
) -> String {
    let mut h = String::from("{\"Authorization\":\"");
    h.push_str(auth);
    h.push_str("\",\"host\":\"");
    h.push_str(host_val);
    h.push_str("\",\"x-sdk-date\":\"");
    h.push_str(timestamp);
    h.push_str("\",\"content-type\":\"application/json\"");
    if let Some(token) = security_token {
        if !token.is_empty() {
            h.push_str(",\"X-Security-Token\":\"");
            h.push_str(token);
            h.push('\"');
        }
    }
    h.push('}');
    h
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    let result = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

fn hmac_sha256_owned(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

fn hex(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn utc_nanos_to_timestamp(nanos: u64) -> String {
    let secs = nanos / 1_000_000_000;
    let days = (secs / 86400) as i32;
    let time_secs = (secs % 86400) as i32;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;

    let (y, mo, d) = epoch_days_to_date(days);
    let mut out = String::with_capacity(16);
    push_pad4(&mut out, y);
    push_pad2(&mut out, mo);
    push_pad2(&mut out, d);
    out.push('T');
    push_pad2(&mut out, h);
    push_pad2(&mut out, m);
    push_pad2(&mut out, s);
    out.push('Z');
    out
}

fn push_pad4(s: &mut String, n: i32) {
    s.push(HEX[(n / 1000) as usize & 0xf] as char);
    s.push(HEX[(n / 100 % 10) as usize] as char);
    s.push(HEX[(n / 10 % 10) as usize] as char);
    s.push(HEX[(n % 10) as usize] as char);
}

fn push_pad2(s: &mut String, n: i32) {
    s.push(HEX[(n / 10) as usize] as char);
    s.push(HEX[(n % 10) as usize] as char);
}

fn epoch_days_to_date(days: i32) -> (i32, i32, i32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as i32, d as i32)
}
