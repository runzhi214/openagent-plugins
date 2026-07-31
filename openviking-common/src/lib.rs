#![no_std]

extern crate alloc;

use alloc::string::String;

use openagent_pdk::prelude::serde_json;
use serde::Deserialize;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:1933";
const JSON_HEADERS: &str = r#"{"Content-Type":"application/json"}"#;
const KEYRING_SERVICE: &str = "openviking";

// ── API response types ──

#[derive(Deserialize, Default)]
struct CreateSessionResult {
    #[serde(default)]
    session_id: String,
}

#[derive(Deserialize, Default)]
struct CreateSessionResp {
    #[serde(default)]
    result: CreateSessionResult,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize, Default)]
struct AddMessageResp {
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize, Default)]
struct SearchResp {
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize, Default)]
struct CommitResp {
    #[serde(default)]
    error: Option<String>,
}

// ── Public API ──

pub fn get_base_url() -> String {
    match openagent_pdk::host::keyring_get("openviking", "url") {
        Ok(v) if !v.is_empty() => v,
        _ => String::from(DEFAULT_BASE_URL),
    }
}

pub fn get_or_create_session() -> Result<String, String> {
    let oa_sid = openagent_pdk::host::runtime_session_id().unwrap_or_default();
    if !oa_sid.is_empty() {
        if let Ok(id) = openagent_pdk::host::keyring_get(KEYRING_SERVICE, &oa_sid) {
            if !id.is_empty() {
                return Ok(id);
            }
        }
    }
    let base = get_base_url();
    let url = alloc::format!("{}/api/v1/sessions", base);
    let (status, body) = openagent_pdk::host::http_request("POST", &url, JSON_HEADERS, b"{}")?;
    if status != 200 {
        return Err(alloc::format!("create_session: status={}", status));
    }
    let resp: CreateSessionResp = serde_json::from_str(&body)
        .map_err(|e| alloc::format!("create_session: parse: {}", e))?;
    if let Some(e) = resp.error {
        if !e.is_empty() {
            return Err(e);
        }
    }
    let sid = resp.result.session_id;
    if sid.is_empty() {
        return Err(String::from("create_session: empty session_id"));
    }
    if !oa_sid.is_empty() {
        let _ = openagent_pdk::host::keyring_set(KEYRING_SERVICE, &oa_sid, &sid);
    }
    let msg = alloc::format!("openviking: created session {}", sid);
    openagent_pdk::host::log_info(&msg);
    Ok(sid)
}

pub fn add_message(session_id: &str, role: &str, content: &str) -> Result<(), String> {
    let base = get_base_url();
    let url = alloc::format!("{}/api/v1/sessions/{}/messages", base, session_id);
    let body = alloc::format!(r#"{{"role":"{}","content":{}}}"#, role, serde_json::Value::String(String::from(content)));
    let (status, resp_body) = openagent_pdk::host::http_request("POST", &url, JSON_HEADERS, body.as_bytes())?;
    if status != 200 {
        return Err(alloc::format!("add_message: status={}", status));
    }
    let resp: AddMessageResp = serde_json::from_str(&resp_body)
        .map_err(|e| alloc::format!("add_message: parse: {}", e))?;
    if let Some(e) = resp.error {
        if !e.is_empty() {
            return Err(e);
        }
    }
    Ok(())
}

pub fn search(session_id: &str, query: &str, limit: u32) -> Result<String, String> {
    let base = get_base_url();
    let url = alloc::format!("{}/api/v1/search/search", base);
    let body = alloc::format!(
        r#"{{"query":{},"session_id":"{}","limit":{}}}"#,
        serde_json::Value::String(String::from(query)),
        session_id,
        limit
    );
    let (status, resp_body) = openagent_pdk::host::http_request("POST", &url, JSON_HEADERS, body.as_bytes())?;
    if status != 200 {
        return Err(alloc::format!("search: status={}", status));
    }
    let resp: SearchResp = serde_json::from_str(&resp_body)
        .map_err(|e| alloc::format!("search: parse: {}", e))?;
    if let Some(e) = resp.error {
        if !e.is_empty() {
            return Err(e);
        }
    }
    Ok(serde_json::to_string(&resp.result)
        .map_err(|e| alloc::format!("search: serialize: {}", e))?)
}

pub fn commit_session(session_id: &str, keep_recent: u32) -> Result<(), String> {
    let base = get_base_url();
    let url = alloc::format!("{}/api/v1/sessions/{}/commit", base, session_id);
    let body = alloc::format!(r#"{{"keep_recent_count":{}}}"#, keep_recent);
    let (status, resp_body) = openagent_pdk::host::http_request("POST", &url, JSON_HEADERS, body.as_bytes())?;
    if status != 200 {
        return Err(alloc::format!("commit: status={}", status));
    }
    let resp: CommitResp = serde_json::from_str(&resp_body)
        .map_err(|e| alloc::format!("commit: parse: {}", e))?;
    if let Some(e) = resp.error {
        if !e.is_empty() {
            return Err(e);
        }
    }
    Ok(())
}

pub fn track_tokens_and_maybe_commit(content: &str, threshold: u32, keep_recent: u32) {
    let added = (content.len() / 4) as u32;
    let oa_sid = openagent_pdk::host::runtime_session_id().unwrap_or_default();
    let token_key = alloc::format!("tokens_{}", oa_sid);
    let current = openagent_pdk::host::keyring_get(KEYRING_SERVICE, &token_key)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let total = current.saturating_add(added);
    let _ = openagent_pdk::host::keyring_set(KEYRING_SERVICE, &token_key, &alloc::format!("{}", total));
    if total >= threshold {
        let _ = openagent_pdk::host::keyring_set(KEYRING_SERVICE, &token_key, "0");
        match get_or_create_session() {
            Ok(sid) => {
                match commit_session(&sid, keep_recent) {
                    Ok(()) => openagent_pdk::host::log_info(&alloc::format!(
                        "openviking: committed session {} at {} tokens",
                        sid, total
                    )),
                    Err(e) => openagent_pdk::host::log_warn(&alloc::format!(
                        "openviking: commit failed: {}", e
                    )),
                }
            }
            Err(e) => openagent_pdk::host::log_warn(&alloc::format!(
                "openviking: commit: cannot get session: {}", e
            )),
        }
    }
}

pub fn extract_text(role: &str, content: &str, content_parts: &[openagent_pdk::types::ContentPart]) -> Option<String> {
    if role == "tool" {
        return None;
    }
    if !content.is_empty() {
        return Some(String::from(content));
    }
    let mut text = String::new();
    for part in content_parts {
        if part.r#type == "text" && !part.text.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&part.text);
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
