#![no_std]
#![no_main]

extern crate alloc;
use openagent_pdk::export::Plugin;
use openagent_pdk::prelude::*;

use hdspace_common::{escape_json, get_models, load_aksk, PROVIDER, report_event, EVENT_LLM_401, EVENT_AKSK_MISSING};

const TRIGGER_ERROR: &str = "401";

struct HdspaceRenewPlugin;

impl Plugin for HdspaceRenewPlugin {
    fn plugin_type() -> &'static str {
        "agent:observers"
    }
    fn name() -> &'static str {
        "hdspace-renew"
    }
    fn description() -> &'static str {
        "On HTTP auth error, renew Huawei Cloud model configs from keyring AKSK via TokenHub"
    }

    fn stage_filter() -> (&'static str, &'static str) {
        ("model.call", "leave")
    }

    fn observe_stage(event: &StageInput) -> StageOutput {
        if event.phase == "leave" && !event.error.is_empty() && event.error.contains(TRIGGER_ERROR)
        {
            let pid = host::runtime_provider().unwrap_or_default();
            if pid != PROVIDER {
                host::log_info(&alloc::format!(
                    "hdspace-renew: provider={} != {}, skipping renew",
                    pid,
                    PROVIDER
                ));
                return StageOutput {
                    action: String::from("continue"),
                    reason: String::new(),
                };
            }
            host::log_warn(&alloc::format!(
                "hdspace-renew: {} detected, renewing model configs",
                TRIGGER_ERROR
            ));
            report_event(EVENT_LLM_401, "LLM call returned 401 auth error");

            let aksk = match load_aksk() {
                Some(a) => a,
                None => {
                    host::log_warn("hdspace-renew: AKSK not found, skipping renew");
                    report_event(
                        EVENT_AKSK_MISSING,
                        "Failed to read HW_ACCESS_KEY or HW_SECRET_KEY from keyring when renewing",
                    );
                    return StageOutput {
                        action: String::from("continue"),
                        reason: String::new(),
                    };
                }
            };

            host::log_info("hdspace-renew: AKSK found, fetching fresh model config");

            match get_models(&aksk.ak, &aksk.sk, aksk.security_token.as_deref()) {
                Some(cfg) => {
                    host::log_info(&alloc::format!(
                        "hdspace-renew: got {} models, updating configs",
                        cfg.models.len()
                    ));
                    let esc_api_key = escape_json(&cfg.api_key);
                    let esc_base_url = escape_json(&cfg.base_url);
                    for m in &cfg.models {
                        let esc_model_id = escape_json(&m.model_id);
                        let cfg_json = alloc::format!(
                            r#"{{"provider":"{}","model_id":"{}","api_key":"{}","base_url":"{}","max_input_tokens":{},"max_output_tokens":{}}}"#,
                            PROVIDER,
                            esc_model_id,
                            esc_api_key,
                            esc_base_url,
                            m.context_window,
                            m.max_tokens
                        );
                        match host::runtime_set_model_config(&cfg_json) {
                            Ok(()) => host::log_info(&alloc::format!(
                                "hdspace-renew: updated model {}",
                                m.model_id
                            )),
                            Err(e) => host::log_error(&alloc::format!(
                                "hdspace-renew: failed to update model {}: {}",
                                m.model_id,
                                e
                            )),
                        }
                    }
                    host::log_info("hdspace-renew: model configs renewed");

                    if let Some(em) = cfg.embedding_models.first() {
                        let emb_json = alloc::format!(
                            r#"{{"base_url":"{}","api_key":"{}","model":"{}"}}"#,
                            esc_base_url,
                            esc_api_key,
                            escape_json(&em.model_id)
                        );
                        match host::runtime_set_embedding_config(&emb_json) {
                            Ok(()) => host::log_info(&alloc::format!(
                                "hdspace-renew: updated embedding model {}",
                                em.model_id
                            )),
                            Err(e) => host::log_error(&alloc::format!(
                                "hdspace-renew: failed to update embedding model {}: {}",
                                em.model_id,
                                e
                            )),
                        }
                    }
                }
                None => {
                    host::log_warn("hdspace-renew: failed to fetch model config from TokenHub");
                }
            }
        }

        StageOutput {
            action: String::from("continue"),
            reason: String::new(),
        }
    }
}

openagent_pdk::export!(HdspaceRenewPlugin);
