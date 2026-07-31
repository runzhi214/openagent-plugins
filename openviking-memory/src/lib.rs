#![no_std]
#![no_main]

extern crate alloc;
use openagent_pdk::export::Plugin;
use openagent_pdk::prelude::*;
use openviking_common::{add_message, get_base_url};

struct OpenVikingMemoryPlugin;

impl Plugin for OpenVikingMemoryPlugin {
    fn plugin_type() -> &'static str { "agent:memory" }
    fn name() -> &'static str { "openviking-memory" }
    fn description() -> &'static str {
        "OpenViking L3 archive search + message mirroring for semantic indexing"
    }

    fn memory_init(config: &serde_json::Value) -> Result<(), String> {
        if let Some(url) = config.get("server_url").and_then(|v| v.as_str()) {
            if !url.is_empty() {
                let _ = openagent_pdk::host::keyring_set("openviking", "url", url);
            }
        }
        if let Some(key) = config.get("api_key").and_then(|v| v.as_str()) {
            if !key.is_empty() {
                let _ = openagent_pdk::host::keyring_set("openviking", "api_key", key);
            }
        }
        let _ = get_base_url();
        openagent_pdk::host::log_info("openviking-memory: initialized");
        Ok(())
    }

    fn memory_append(input: &MemoryAppendInput) -> Result<(), String> {
        let msg = &input.message;
        let text = openviking_common::extract_text(
            &msg.role,
            &msg.content,
            &msg.content_parts,
        );
        let content = match text {
            Some(t) => t,
            None => return Ok(()),
        };

        add_message(&input.session_id, &msg.role, &content)?;
        openagent_pdk::host::log_info(&alloc::format!(
            "openviking-memory: append role={} session={}",
            msg.role, input.session_id
        ));
        Ok(())
    }

    fn memory_search(input: &MemorySearchInput) -> Result<alloc::vec::Vec<SearchResult>, String> {
        let limit = if input.limit > 0 { input.limit as u32 } else { 10 };
        let raw = openviking_common::search(&input.session_id, &input.query, limit)?;
        let results = parse_search_results(&raw);
        openagent_pdk::host::log_info(&alloc::format!(
            "openviking-memory: search query={} results={} session={}",
            input.query, results.len(), input.session_id
        ));
        Ok(results)
    }
}

fn parse_search_results(raw: &str) -> alloc::vec::Vec<SearchResult> {
    let val = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => v,
        Err(_) => return alloc::vec::Vec::new(),
    };

    let mut results = alloc::vec::Vec::new();

    if let Some(memories) = val.get("memories").and_then(|v| v.as_array()) {
        for m in memories {
            let score = m.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let abstract_text = m
                .get("overview")
                .and_then(|v| v.as_str())
                .or_else(|| m.get("abstract").and_then(|v| v.as_str()))
                .unwrap_or("");
            if abstract_text.is_empty() {
                continue;
            }
            results.push(SearchResult {
                message: Message {
                    role: String::from("assistant"),
                    content: String::from(abstract_text),
                    ..Default::default()
                },
                score,
                turn: 0,
            });
        }
    }

    if let Some(resources) = val.get("resources").and_then(|v| v.as_array()) {
        for r in resources {
            let score = r.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let abstract_text = r.get("abstract").and_then(|v| v.as_str()).unwrap_or("");
            if abstract_text.is_empty() {
                continue;
            }
            results.push(SearchResult {
                message: Message {
                    role: String::from("assistant"),
                    content: String::from(abstract_text),
                    ..Default::default()
                },
                score,
                turn: 0,
            });
        }
    }

    results
}

openagent_pdk::export!(OpenVikingMemoryPlugin);
