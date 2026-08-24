#![no_std]
extern crate alloc;
extern crate openagent_pdk as sdk;
use sdk::export::Plugin;
use sdk::prelude::*;

use hdspace_common::{escape_json, get_models, load_aksk, PROVIDER, report_event, EVENT_AKSK_MISSING};

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
    let aksk = match load_aksk() {
        Some(a) => a,
        None => {
            host::log_info(
                "hdspace-models: AKSK not set, returning settings as-is",
            );
            report_event(
                EVENT_AKSK_MISSING,
                "Failed to read HW_ACCESS_KEY or HW_SECRET_KEY from keyring when fetching models",
            );
            return String::from(settings);
        }
    };

    host::log_info("hdspace-models: AKSK found, fetching model config");

    let config = get_models(&aksk.ak, &aksk.sk, aksk.security_token.as_deref());
    merge_settings(
        settings,
        config.as_ref(),
        &aksk.ak,
        &aksk.sk,
        aksk.security_token.as_deref(),
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
        let esc_api_key = escape_json(&cfg.api_key);
        let esc_base_url = escape_json(&cfg.base_url);

        out.push_str("\"provider\":{\"");
        out.push_str(PROVIDER);
        out.push_str("\":{\"api_key\":\"");
        out.push_str(&esc_api_key);
        out.push_str("\",\"base_url\":\"");
        out.push_str(&esc_base_url);
        out.push_str("\",\"models\":[");

        let mut first = true;
        for m in &cfg.models {
            if !first {
                out.push(',')
            } else {
                first = false
            }
            out.push_str("{\"id\":\"");
            out.push_str(&escape_json(&m.model_id));
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
                out.push_str(&escape_json(&first.model_id));
            }
            out.push_str("\",");
            host::log_info("hdspace-models: default model injected");
        }
        host::log_info("hdspace-models: provider config injected");

        if !cfg.embedding_models.is_empty() {
            out.push_str("\"embedding\":{\"provider\":\"openai\",\"base_url\":\"");
            out.push_str(&esc_base_url);
            out.push_str("\",\"api_key\":\"");
            out.push_str(&esc_api_key);
            out.push_str("\",\"model\":\"");
            out.push_str(&escape_json(&cfg.embedding_models[0].model_id));
            out.push_str("\"},");
            host::log_info("hdspace-models: embedding config injected");
        }
    }

    out.push_str("\"env\":{\"HW_ACCESS_KEY\":\"");
    out.push_str(&escape_json(ak));
    out.push_str("\",\"HW_SECRET_KEY\":\"");
    out.push_str(&escape_json(sk));
    out.push('\"');

    if let Some(token) = security_token {
        if !token.is_empty() {
            out.push_str(",\"HW_SECURITY_TOKEN\":\"");
            out.push_str(&escape_json(token));
            out.push('\"');
        }
    }

    out.push_str("}}");
    out
}
