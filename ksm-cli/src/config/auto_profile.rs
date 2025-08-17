use anyhow::Result;
use regex::Regex;
use std::env;

use super::types::AutoProfileRule;

pub fn select_auto_profile(auto_profile_rules: &[AutoProfileRule]) -> Result<Option<String>> {
    for rule in auto_profile_rules {
        if rule_matches(rule)? {
            return Ok(Some(rule.profile.clone()));
        }
    }
    Ok(None)
}

fn rule_matches(rule: &AutoProfileRule) -> Result<bool> {
    // Check hostname regex
    if let Some(ref hostname_regex) = rule.hostname_regex {
        if let Ok(hostname) = hostname::get() {
            let regex = Regex::new(hostname_regex)?;
            if regex.is_match(&hostname.to_string_lossy()) {
                return Ok(true);
            }
        }
    }

    // Check environment variables
    if let Some(ref env_vars) = rule.env {
        for (key, expected_value) in env_vars {
            if let Ok(actual_value) = env::var(key) {
                if &actual_value == expected_value {
                    return Ok(true);
                }
            }
        }
    }

    // Check SSH session
    if let Some(ssh_session) = rule.ssh_session {
        let is_ssh = env::var("SSH_CLIENT").is_ok() || env::var("SSH_TTY").is_ok();
        if ssh_session == is_ssh {
            return Ok(true);
        }
    }

    // Check default rule
    if rule.default.unwrap_or(false) {
        return Ok(true);
    }

    Ok(false)
}
