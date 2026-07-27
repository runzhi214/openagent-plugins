#![no_std]
#![no_main]

extern crate alloc;
use openagent_pdk::export::Plugin;
use openagent_pdk::prelude::*;

use hdspace_common::{get_models, PROVIDER};

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

            let ak = match host::keyring_get("openagent", "HW_ACCESS_KEY") {
                Ok(v) if !v.is_empty() => v,
                _ => {
                    host::log_warn("hdspace-renew: HW_ACCESS_KEY not found, skipping renew");
                    return StageOutput {
                        action: String::from("continue"),
                        reason: String::new(),
                    };
                }
            };
            let sk = match host::keyring_get("openagent", "HW_SECRET_KEY") {
                Ok(v) if !v.is_empty() => v,
                _ => {
                    host::log_warn("hdspace-renew: HW_SECRET_KEY not found, skipping renew");
                    return StageOutput {
                        action: String::from("continue"),
                        reason: String::new(),
                    };
                }
            };
            let security_token = host::keyring_get("openagent", "HW_SECURITY_TOKEN")
                .ok()
                .filter(|v| !v.is_empty());

            host::log_info("hdspace-renew: AKSK found, fetching fresh model config");

            match get_models(&ak, &sk, security_token.as_deref()) {
                Some(cfg) => {
                    host::log_info(&alloc::format!(
                        "hdspace-renew: got {} models, updating configs",
                        cfg.models.len()
                    ));
                    for m in &cfg.models {
                        let cfg_json = alloc::format!(
                            r#"{{"provider":"{}","model_id":"{}","api_key":"{}","base_url":"{}"}}"#,
                            PROVIDER,
                            m.model_id,
                            cfg.api_key,
                            cfg.base_url
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
