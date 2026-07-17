// extended-settings — reads provider credentials from keyring
// and injects them into settings.
//
// Build:
//   cargo build --release --target wasm32-unknown-unknown -p extended-settings

#![no_std]
extern crate openagent_cli_sdk as sdk;
use sdk::prelude::*;

static META: &str = r#"{"type":"cli:settings","name":"extended-settings","description":"Injects provider credentials from keyring into settings"}"#;

#[no_mangle] pub extern "C" fn alloc(size: u32) -> u32 { sdk_alloc(size) }
#[no_mangle] pub extern "C" fn metadata() -> u64 { sdk_meta(META) }

#[no_mangle]
pub extern "C" fn init(p: u32, l: u32) -> u64 {
    host::log_info("extended-settings: init called");
    let s = unsafe { wasm_str(p, l) };
    let result = cli_init(s);
    sdk_return(result.as_bytes())
}

fn cli_init(settings: &str) -> String {
    let ak = host::keyring_get("openagent", "my_provider_api_key");
    let bu = host::keyring_get("openagent", "my_provider_base_url");
    if ak.is_none() || bu.is_none() {
        host::log_info("extended-settings: keyring miss, returning settings as-is");
        return String::from(settings);
    }
    host::log_info("extended-settings: keyring hit, injecting provider config");
    let ak = ak.unwrap();
    let bu = bu.unwrap();
    let mdls = host::keyring_get("openagent", "my_provider_models");

    let trimmed = settings.trim_end();
    let end = if trimmed.ends_with('}') { trimmed.len() - 1 } else { trimmed.len() };

    let mut out = String::from(&settings[..end]);
    if settings[..end].trim_end() != "{" {
        out.push(',');
    }
    out.push_str("\"provider\":{\"my_provider\":{\"api_key\":\"");
    out.push_str(ak);
    out.push_str("\",\"base_url\":\"");
    out.push_str(bu);
    out.push_str("\",\"models\":[");

    if let Some(mdls) = mdls {
        let mut first = true;
        for m in mdls.split(',') {
            let m = m.trim();
            if m.is_empty() { continue }
            if !first { out.push(',') } else { first = false }
            out.push('\"');
            out.push_str(m);
            out.push('\"');
        }
    }
    out.push_str("]}}");

    out.push_str(",\"env\":{\"MY_PROVIDER_API_KEY\":\"");
    out.push_str(ak);
    out.push_str("\"}}");

    out
}
