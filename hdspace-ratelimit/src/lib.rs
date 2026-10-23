// TokenHub rate-limit hint — replaces the 429 error message with
// user-facing guidance (markdown: emoji, bold, site-relative link).
//
// The host consults on_api_error whenever a provider API call fails with
// a status code; we answer only for 429 from TokenHub gateways.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use openagent_pdk::export::Plugin;
use openagent_pdk::prelude::*;

const RATE_LIMIT_MSG: &str = "✅ 当前使用人数较多, 请求被临时限流, 通常几分钟内自动恢复, 可稍后重试. 不想等待? 点击 [自定义模型](/settings?q=model), 配置专属模型继续.";

// TokenHub base_url hosts (settings stores the /v2 root; the SDK appends
// /chat/completions, so match on the host substring, never the full URL).
const TOKENHUB_HOSTS: &[&str] = &[
    "tokenhub.developer.huaweicloud.com",
    "devstation.myhuaweicloud.com",
];

struct HdspaceRatelimitPlugin;

impl Plugin for HdspaceRatelimitPlugin {
    fn plugin_type() -> &'static str {
        "agent:observers"
    }
    fn name() -> &'static str {
        "hdspace-ratelimit"
    }
    fn description() -> &'static str {
        "Replaces TokenHub 429 rate-limit errors with actionable guidance"
    }

    // Decorator consults only — no stage work. The narrow filter keeps
    // the host from invoking this plugin's run() on unrelated stages.
    fn stage_filter() -> (&'static str, &'static str) {
        ("model.call", "leave")
    }

    fn on_api_error(err: &ApiErrorInput) -> String {
        if err.status_code == 429 && TOKENHUB_HOSTS.iter().any(|h| err.base_url.contains(h)) {
            host::log_info("hdspace-ratelimit: TokenHub 429, replacing message");
            String::from(RATE_LIMIT_MSG)
        } else {
            String::new()
        }
    }
}

openagent_pdk::export!(HdspaceRatelimitPlugin);
