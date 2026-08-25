#![no_std]
extern crate alloc;
extern crate openagent_pdk as sdk;
use sdk::export::Plugin;
use sdk::prelude::*;

use hdspace_common::{get_models, report_event, EVENT_AKSK_MISSING, PROVIDER};

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
    let ak = match host::keyring_get("hwcloud", "HW_ACCESS_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            host::log_info(
                "hdspace-models: HW_ACCESS_KEY or HW_SECRET_KEY not set, returning settings as-is",
            );
            report_event(
                EVENT_AKSK_MISSING,
                "Failed to read HW_ACCESS_KEY from keyring when fetching models",
            );
            return String::from(settings);
        }
    };
    let sk = match host::keyring_get("hwcloud", "HW_SECRET_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            host::log_info(
                "hdspace-models: HW_ACCESS_KEY or HW_SECRET_KEY not set, returning settings as-is",
            );
            report_event(
                EVENT_AKSK_MISSING,
                "Failed to read HW_SECRET_KEY from keyring when fetching models",
            );
            return String::from(settings);
        }
    };
    let security_token = host::keyring_get("hwcloud", "HW_SECURITY_TOKEN")
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

fn merge_settings(
    settings: &str,
    config: Option<&hdspace_common::AgentConfig>,
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
        out.push_str("\"provider\":{\"");
        out.push_str(PROVIDER);
        out.push_str("\":{\"api_key\":\"");
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
            out.push_str("{\"id\":\"");
            out.push_str(&m.model_id);
            out.push_str("\",\"max_input_tokens\":");
            out.push_str(&alloc::format!("{}", m.context_window));
            out.push_str(",\"max_output_tokens\":");
            out.push_str(&alloc::format!("{}", m.max_tokens));
            out.push('}');
        }
        out.push_str("]}},");
        if !settings.contains("\"model\"") {
            out.push_str("\"model\":\"");
            out.push_str(PROVIDER);
            out.push('/');
            if let Some(first) = cfg.models.first() {
                out.push_str(&first.model_id);
            }
            out.push_str("\",");
            host::log_info("hdspace-models: default model injected");
        }
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
