use std::path::PathBuf;

use crate::error::TaError;

/// Install agent hooks for automatic status tracking.
/// Currently supports Claude Code.
pub fn setup_hooks() -> Result<(), TaError> {
    println!("Setting up agent hooks for ta...\n");

    let mut any_installed = false;

    if let Some(result) = setup_claude_hooks()? {
        any_installed = true;
        println!("  Claude Code: {}", result);
    }

    if !any_installed {
        println!("  No supported agents found.");
        println!("  Supported: Claude Code (~/.claude/settings.json)");
    }

    println!();
    println!("Hooks call: ta set-window-status <working|waiting|done>");
    println!("To delegate to workmux instead, use:");
    println!("  ta set-window-status <status> --command 'workmux set-window-status'");

    Ok(())
}

/// The hook entries we want in Claude Code's settings.json.
fn ta_hooks() -> serde_json::Value {
    serde_json::json!({
        "hooks": {
            "UserPromptSubmit": [
                {
                    "hooks": [{
                        "type": "command",
                        "command": "ta set-window-status working"
                    }]
                }
            ],
            "Notification": [
                {
                    "matcher": "permission_prompt|elicitation_dialog",
                    "hooks": [{
                        "type": "command",
                        "command": "ta set-window-status waiting"
                    }]
                }
            ],
            "PostToolUse": [
                {
                    "hooks": [{
                        "type": "command",
                        "command": "ta set-window-status working"
                    }]
                }
            ],
            "Stop": [
                {
                    "hooks": [{
                        "type": "command",
                        "command": "ta set-window-status done"
                    }]
                }
            ]
        }
    })
}

fn claude_settings_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".claude").join("settings.json"))
}

fn setup_claude_hooks() -> Result<Option<String>, TaError> {
    let Some(settings_path) = claude_settings_path() else {
        return Ok(None);
    };

    // Check if Claude Code is installed
    if !settings_path.parent().is_some_and(|p| p.exists()) {
        return Ok(None);
    }

    // Load existing settings or create new
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let ta_hook_config = ta_hooks();
    let ta_hooks_map = ta_hook_config["hooks"].as_object().unwrap();

    // Check if ta hooks are already installed
    let existing_hooks = settings
        .get("hooks")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let already_installed = ta_hooks_map.iter().all(|(event, entries)| {
        if let Some(existing_entries) = existing_hooks.get(event).and_then(|v| v.as_array()) {
            let new_entries = entries.as_array().unwrap();
            new_entries.iter().all(|new_entry| {
                existing_entries
                    .iter()
                    .any(|existing| existing == new_entry)
            })
        } else {
            false
        }
    });

    if already_installed {
        return Ok(Some("hooks already installed".to_string()));
    }

    // Merge hooks into settings
    let hooks = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert(serde_json::json!({}));

    for (event, new_entries) in ta_hooks_map {
        let event_hooks = hooks
            .as_object_mut()
            .unwrap()
            .entry(event)
            .or_insert(serde_json::json!([]));

        if let Some(arr) = event_hooks.as_array_mut() {
            let new_arr = new_entries.as_array().unwrap();
            for entry in new_arr {
                // Deduplicate: don't add if an identical entry exists
                if !arr.iter().any(|existing| existing == entry) {
                    arr.push(entry.clone());
                }
            }
        }
    }

    // Write back
    let content = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&settings_path, content)?;

    Ok(Some("hooks installed".to_string()))
}
