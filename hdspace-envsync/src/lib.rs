//! hdspace-envsync — scheduled-job plugin.
//!
//! Every 1 minutes, reads HW_ACCESS_KEY / HW_SECRET_KEY / HW_SECURITY_TOKEN
//! from the system keyring (service "hwcloud") and syncs them into the
//! host process environment so downstream consumers (iac-server, terraform
//! subprocess, SDK-HMAC-SHA256 signing) see fresh credentials without a
//! restart.

#![no_std]
#![no_main]

extern crate alloc;
use openagent_pdk::export::Plugin;
use openagent_pdk::prelude::*;

pub struct HdspaceEnvSyncPlugin;

impl Plugin for HdspaceEnvSyncPlugin {
    fn name() -> &'static str {
        "hdspace-envsync"
    }
    fn description() -> &'static str {
        "every 1 min: sync HW_ACCESS_KEY/HW_SECRET_KEY/HW_SECURITY_TOKEN from keyring into host env"
    }

    fn scheduled_jobs() -> Vec<ScheduledJob> {
        vec![ScheduledJob {
            id: "sync-hw-creds".into(),
            cron: "*/1 * * * *".into(),
            description: "sync Huawei AK/SK from keyring to host env".into(),
        }]
    }

    fn run_scheduled_job(job: &ScheduledJobInput) -> Result<String, String> {
        let ak = host::keyring_get("hwcloud", "HW_ACCESS_KEY")?;
        if ak.is_empty() {
            return Err("HW_ACCESS_KEY not found in keyring".into());
        }
        let sk = host::keyring_get("hwcloud", "HW_SECRET_KEY")?;
        if sk.is_empty() {
            return Err("HW_SECRET_KEY not found in keyring".into());
        }

        host::env_set("HW_ACCESS_KEY", &ak)?;
        host::env_set("HW_SECRET_KEY", &sk)?;

        match host::keyring_get("hwcloud", "HW_SECURITY_TOKEN") {
            Ok(t) if !t.is_empty() => host::env_set("HW_SECURITY_TOKEN", &t)?,
            _ => host::env_unset("HW_SECURITY_TOKEN")?,
        }

        Ok(alloc::format!(
            "job {} at {}: synced AK/SK ({}+{} bytes)",
            job.id,
            job.scheduled_at,
            ak.len(),
            sk.len(),
        ))
    }
}

openagent_pdk::export!(HdspaceEnvSyncPlugin);
