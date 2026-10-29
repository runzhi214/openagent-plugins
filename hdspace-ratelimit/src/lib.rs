// TokenHub rate-limit hint — replaces the 429 error message with
// user-facing guidance (markdown: emoji, bold, site-relative link).
//
// Layout: the pure parsing/linking helpers below are host-testable
// (std, no PDK), while everything touching the PDK/host ABI lives in the
// `pdk`-gated plugin module — the PDK is no_std with a panic_handler,
// which cannot link against a std test build.

#![cfg_attr(feature = "pdk", no_std)]
#![cfg_attr(feature = "pdk", no_main)]

#[cfg(feature = "pdk")]
extern crate alloc;

#[cfg(feature = "pdk")]
mod plugin {
    use alloc::string::String;
    use openagent_pdk::export::Plugin;
    use openagent_pdk::prelude::*;

    use super::{parse_provider_error, CUSTOM_MODEL_LINK, QUOTA_EXHAUSTED_CODE};

    const RATE_LIMIT_MSG: &str = "✅ 当前模型接受请求过多触发限流, 您可稍后重试, 或暂时切换使用其他模型, 也可点击 [自定义模型](/settings?q=model) 配置您自己的模型 API-Key 继续使用.";

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
                // OpenAI-compatible gateways append {"code":..,"message":..}
                // to the SDK error text. Code 42901 = personal quota exhausted:
                // relay the gateway's own message with the remedy word
                // hyperlinked. Any parse miss (legacy body shapes) falls
                // through to the fixed hint, so this never regresses the
                // pre-existing behavior.
                if let Some((code, message)) = parse_provider_error(&err.error) {
                    if code == QUOTA_EXHAUSTED_CODE {
                        host::log_info("hdspace-ratelimit: quota exhausted (42901), relaying message");
                        return message.replace("自定义模型", CUSTOM_MODEL_LINK);
                    }
                }
                host::log_info("hdspace-ratelimit: TokenHub 429, replacing message");
                String::from(RATE_LIMIT_MSG)
            } else {
                String::new()
            }
        }
    }

    openagent_pdk::export!(HdspaceRatelimitPlugin);
}

// Quota-exhausted error code from TokenHub: its message names the
// self-service remedy ("自定义模型") which we hyperlink to the settings
// page. Matched as a string; numeric JSON values are normalized by
// parse_provider_error.
const QUOTA_EXHAUSTED_CODE: &str = "42901";

// Link target injected into quota-exhausted messages ("自定义模型" → the
// settings page). Site-relative, rendered by the frontend's markdown.
const CUSTOM_MODEL_LINK: &str = "[自定义模型](/settings?q=model)";

#[cfg(feature = "pdk")]
use alloc::string::{String, ToString};

/// Extracts (code, message) from the host error text. The SDK formats
/// API errors as `POST "url": 429 ... {"code":..,"message":..}` — URLs
/// contain no '{', so the first '{' marks the JSON tail, and the tail
/// runs to the end of the string. Returns None when there is no JSON
/// tail or it fails to parse (legacy/plain-text gateway bodies).
/// A numeric `code` is normalized to its string form so both
/// `"code":"42901"` and `"code":42901` match.
fn parse_provider_error(err_text: &str) -> Option<(String, String)> {
    let start = err_text.find('{')?;
    let value: serde_json::Value = serde_json::from_str(&err_text[start..]).ok()?;
    let code = match value.get("code") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        _ => return None,
    };
    let message = value.get("message")?.as_str()?.to_string();
    Some((code, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SDK_PREFIX: &str =
        "POST \"https://tokenhub.developer.huaweicloud.com/v2/chat/completions\": 429 Too Many Requests ";

    fn quota_err(json_tail: &str) -> String {
        format!("{}{}", SDK_PREFIX, json_tail)
    }

    // 42901 + message containing the remedy: both string and numeric code
    // forms parse to the same (code, message).
    #[test]
    fn parses_string_code() {
        let got = parse_provider_error(&quota_err(
            r#"{"code":"42901","message":"您的共享额度已耗尽, 请配置自定义模型后继续使用"}"#,
        ));
        assert_eq!(
            got,
            Some((
                "42901".to_string(),
                "您的共享额度已耗尽, 请配置自定义模型后继续使用".to_string()
            ))
        );
    }

    #[test]
    fn parses_numeric_code() {
        let got = parse_provider_error(&quota_err(
            r#"{"code":42901,"message":"quota exhausted, configure 自定义模型"}"#,
        ));
        assert_eq!(got, Some(("42901".to_string(), "quota exhausted, configure 自定义模型".to_string())));
    }

    // Other codes parse fine — the on_api_error branch, not the parser,
    // decides they fall back to the fixed hint.
    #[test]
    fn parses_other_code() {
        let got = parse_provider_error(&quota_err(r#"{"code":"too_many_limits","message":"Too Many Limits"}"#));
        assert_eq!(got, Some(("too_many_limits".to_string(), "Too Many Limits".to_string())));
    }

    // Legacy bodies: no JSON tail (the exact shape seen in production
    // before the gateway adopted OpenAI-compatible errors) and malformed
    // JSON both yield None.
    #[test]
    fn no_json_tail_is_none() {
        assert_eq!(parse_provider_error(SDK_PREFIX), None);
        assert_eq!(parse_provider_error(""), None);
    }

    #[test]
    fn malformed_json_is_none() {
        assert_eq!(parse_provider_error(&quota_err(r#"{"code":"42901""#)), None);
    }

    // JSON without code/message fields is not a provider error payload
    // (e.g. APIG-style {"error_code","error_msg"}).
    #[test]
    fn missing_fields_is_none() {
        assert_eq!(parse_provider_error(&quota_err(r#"{"error_code":"APIG.0301","error_msg":"throttled"}"#)), None);
    }

    // Pins the relay gate: the gateway contract the plugin keys on.
    // Also the sole test-build reference keeping the constant non-dead.
    #[test]
    fn quota_exhausted_code_is_42901() {
        assert_eq!(QUOTA_EXHAUSTED_CODE, "42901");
    }

    // The 42901 relay path: remedy word replaced with the settings link.
    // A message without the word passes through verbatim.
    #[test]
    fn link_injection_replaces_remedy_word() {
        let msg = "您的共享额度已耗尽, 请配置自定义模型后继续使用";
        assert_eq!(
            msg.replace("自定义模型", CUSTOM_MODEL_LINK),
            "您的共享额度已耗尽, 请配置[自定义模型](/settings?q=model)后继续使用"
        );
        assert_eq!(
            "额度已耗尽".replace("自定义模型", CUSTOM_MODEL_LINK),
            "额度已耗尽"
        );
    }
}
