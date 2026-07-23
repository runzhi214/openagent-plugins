#![no_std]
extern crate alloc;
extern crate openagent_pdk as sdk;
use sdk::prelude::*;
use sdk::export::Plugin;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
type HmacSha256 = Hmac<Sha256>;

use serde::Deserialize;

const DEFAULT_DOMAIN: &str = "devstation.myhuaweicloud.com";
const API_PATH: &str = "/open-api-public/v1/tokenhub-configs";
const API_PATH_SIGN: &str = "/open-api-public/v1/tokenhub-configs/";
const HEX: &[u8; 16] = b"0123456789abcdef";

struct HdspacePlugin;

impl Plugin for HdspacePlugin {
    fn plugin_type() -> &'static str {
        "cli:settings"
    }
    fn name() -> &'static str {
        "hdspace-models"
    }
    fn description() -> &'static str {
        "Huawei Cloud Space models via AKSK"
    }
    fn init(settings: &str) -> Result<String, String> {
        host::log_info("hdspace-models: init called");
        Ok(cli_init(settings))
    }
}

openagent_pdk::export!(HdspacePlugin);

fn cli_init(settings: &str) -> String {
    let ak = match host::keyring_get("openagent", "HW_ACCESS_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            host::log_info(
                "hdspace-models: HW_ACCESS_KEY or HW_SECRET_KEY not set, returning settings as-is",
            );
            return String::from(settings);
        }
    };
    let sk = match host::keyring_get("openagent", "HW_SECRET_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            host::log_info(
                "hdspace-models: HW_ACCESS_KEY or HW_SECRET_KEY not set, returning settings as-is",
            );
            return String::from(settings);
        }
    };
    let security_token = host::keyring_get("openagent", "HW_SECURITY_TOKEN")
        .ok()
        .filter(|v| !v.is_empty());

    host::log_info("hdspace-models: AKSK found, fetching model config");

    let config = get_models(&ak, &sk, security_token.as_deref());
    merge_settings(
        settings,
        config.as_ref(),
        &ak,
        &sk,
        security_token.as_deref(),
    )
}

fn get_models(ak: &str, sk: &str, security_token: Option<&str>) -> Option<AgentConfig> {
    let ts = utc_nanos_to_timestamp(host::utc_now());

    let domain = DEFAULT_DOMAIN;
    let url = alloc::format!("https://{}{}", domain, API_PATH);

    let auth = sign_request(ak, sk, domain, API_PATH_SIGN, &ts);
    let headers = build_headers(&auth, domain, &ts, security_token);

    host::log_info("hdspace-models: calling API");
    let url_msg = alloc::format!("hdspace-models: url={}", url);
    host::log_info(&url_msg);

    let (status, body) = match host::http_request("GET", &url, &headers, &[]) {
        Ok(r) => r,
        Err(e) => {
            let warn = alloc::format!("hdspace-models: http error={}", e);
            host::log_warn(&warn);
            return None;
        }
    };

    if status != 200 {
        let warn = alloc::format!("hdspace-models: API status={}", status);
        host::log_warn(&warn);
        let body_msg = alloc::format!("hdspace-models: response body={}", body);
        host::log_warn(&body_msg);
        return None;
    }

    let resp: ApiResponse = serde_json::from_str(&body).ok()?;
    if resp.error_code != "0000" {
        host::log_warn("hdspace-models: API error code not 0000");
        return None;
    }
    Some(resp.result)
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

#[derive(Deserialize)]
struct ModelInfo {
    pub model_id: String,
    #[allow(dead_code)]
    pub model_name: String,
}

#[derive(Deserialize)]
struct AgentConfig {
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

fn merge_settings(
    settings: &str,
    config: Option<&AgentConfig>,
    ak: &str,
    sk: &str,
    security_token: Option<&str>,
) -> String {
    let trimmed = settings.trim_end();
    let end = if trimmed.ends_with('}') {
        trimmed.len() - 1
    } else {
        trimmed.len()
    };

    let mut out = String::with_capacity(settings.len() + 2048);
    out.push_str(&settings[..end]);

    let content = settings[..end].trim_end();
    if !content.is_empty() && !content.ends_with('{') {
        out.push(',');
    }

    if let Some(cfg) = config {
        out.push_str("\"provider\":{\"huawei-free\":{\"api_key\":\"");
        out.push_str(&cfg.api_key);
        out.push_str("\",\"base_url\":\"");
        out.push_str(&cfg.base_url);
        out.push_str("\",\"models\":[");

        let mut first = true;
        for m in &cfg.models {
            if !first {
                out.push(',')
            } else {
                first = false
            }
            out.push('\"');
            out.push_str(&m.model_id);
            out.push('\"');
        }
        out.push_str("]}},");
        host::log_info("hdspace-models: provider config injected");
    }

    out.push_str("\"env\":{\"HW_ACCESS_KEY\":\"");
    out.push_str(ak);
    out.push_str("\",\"HW_SECRET_KEY\":\"");
    out.push_str(sk);
    out.push('\"');

    if let Some(token) = security_token {
        if !token.is_empty() {
            out.push_str(",\"HW_SECURITY_TOKEN\":\"");
            out.push_str(token);
            out.push('\"');
        }
    }

    out.push_str("}}");
    out
}
