use crate::cli::AlertsArgs;
use crate::error::TaError;
use crate::tmux::pane::{parse_pane_title, AgentType};
use crate::tmux::TmuxClient;

const SEP: &str = "|||";

/// Alertable status icons (waiting, done, error, rate-limited).
/// Working (🤖) and empty are not alertable.
const ALERTABLE_ICONS: &[&str] = &["💬", "✅", "❌", "🚫"];

// Catppuccin mocha palette (hardcoded to avoid extra tmux queries).
const THM_CRUST: &str = "#11111b";
const THM_SURFACE_0: &str = "#313244";
const THM_FG: &str = "#cdd6f4";
const THM_MAUVE: &str = "#cba6f7";
const THM_GREEN: &str = "#a6e3a1";
const THM_RED: &str = "#f38ba8";

/// Module accent color per status icon.
fn accent_color(icon: &str) -> &'static str {
    match icon {
        "💬" => THM_MAUVE,
        "✅" => THM_GREEN,
        "❌" | "🚫" => THM_RED,
        _ => THM_MAUVE,
    }
}

struct AlertEntry {
    icon: String,
    label: String,
}

/// Query all windows, filter to alertable, format for tmux status-right.
pub async fn run(client: &TmuxClient, args: &AlertsArgs) -> Result<(), TaError> {
    let format = [
        "#{session_name}",
        "#{window_index}",
        "#{window_name}",
        "#{pane_title}",
        "#{@workmux_status}",
        "#{window_active}",
        "#{session_attached}",
    ]
    .join(SEP);

    let output = client
        .run(&["list-windows", "-a", "-F", &format])
        .await?;

    let mut entries: Vec<AlertEntry> = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split(SEP).collect();
        if parts.len() < 7 {
            continue;
        }

        let session = parts[0];
        let window_index = parts[1];
        let window_name = parts[2];
        let pane_title = parts[3];
        let status_icon = parts[4].trim();
        let window_active = parts[5] == "1";
        let session_attached = parts[6] == "1";

        // Skip non-alertable
        if !ALERTABLE_ICONS.contains(&status_icon) {
            continue;
        }

        // Auto-clear: suppress if user is currently viewing this window
        if window_active && session_attached {
            continue;
        }

        let label = resolve_label(session, window_index, window_name, pane_title);
        entries.push(AlertEntry {
            icon: status_icon.to_string(),
            label,
        });
    }

    if entries.is_empty() {
        return Ok(());
    }

    let formatted = format_entries(&entries, args.max_width);
    print!("{}", formatted);

    Ok(())
}

/// Resolve a compact label for the alert entry.
fn resolve_label(session: &str, window_index: &str, window_name: &str, pane_title: &str) -> String {
    // Try window_name first, then pane_title
    for source in [window_name, pane_title] {
        let (agent_type, index, _, _) = parse_pane_title(source);
        if agent_type != AgentType::User {
            return format!("{}_{}", agent_type.tag(), index);
        }
    }
    // Fallback
    format!("{}:{}", session, window_index)
}

/// Format entries as catppuccin-style pill modules for tmux status-right.
///
/// Each entry renders as a rounded pill matching the catppuccin module style:
///   (left-sep)(icon on accent bg)(mid-sep)(label on surface_0 bg)(right-sep)
fn format_entries(entries: &[AlertEntry], max_width: usize) -> String {
    let mut parts: Vec<String> = Vec::new();

    for entry in entries {
        parts.push(format_pill(&entry.icon, &entry.label));
    }

    let result = parts.join("");

    // Check visible width, truncate if needed
    let visible = strip_tmux_styles(&result);
    if visible_width(&visible) > max_width && parts.len() > 1 {
        while parts.len() > 1 {
            parts.pop();
            let candidate = format!("{}{}", parts.join(""), format_overflow_pill(entries.len() - parts.len()));
            let vis = strip_tmux_styles(&candidate);
            if visible_width(&vis) <= max_width {
                return candidate;
            }
        }
        format!("{}{}", parts.join(""), format_overflow_pill(entries.len() - 1))
    } else {
        result
    }
}

// Powerline glyphs for catppuccin rounded pill style.
const LEFT_SEP: &str = "\u{e0b6}";  //
const MID_SEP: &str = "\u{e0b4}";   //
const RIGHT_SEP: &str = "\u{e0b4}"; //

/// Render a single alert as a catppuccin-style rounded pill.
fn format_pill(icon: &str, label: &str) -> String {
    let accent = accent_color(icon);

    // Left rounded separator: accent-colored on default bg
    // Icon section: crust text on accent bg
    // Middle separator: transitions accent → surface_0
    // Text section: fg text on surface_0 bg
    // Right rounded separator: surface_0 on default bg
    format!(
        "#[fg={accent},bg=default]{left}\
         #[fg={crust},bg={accent}] {icon} \
         #[fg={accent},bg={surface}]{mid}\
         #[fg={fg},bg={surface}] {label} \
         #[fg={surface},bg=default]{right}",
        accent = accent,
        crust = THM_CRUST,
        surface = THM_SURFACE_0,
        fg = THM_FG,
        icon = icon,
        label = label,
        left = LEFT_SEP,
        mid = MID_SEP,
        right = RIGHT_SEP,
    )
}

/// Render an overflow indicator pill showing how many more alerts exist.
fn format_overflow_pill(remaining: usize) -> String {
    format_pill("💬", &format!("+{}", remaining))
}

/// Strip `#[...]` tmux style tags for width measurement.
fn strip_tmux_styles(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '#' && chars.peek() == Some(&'[') {
            in_tag = true;
            chars.next(); // consume '['
            continue;
        }
        if in_tag {
            if ch == ']' {
                in_tag = false;
            }
            continue;
        }
        result.push(ch);
    }
    result
}

/// Approximate visible width. Emoji = 2 columns, ASCII = 1.
fn visible_width(s: &str) -> usize {
    s.chars()
        .map(|c| if c.len_utf8() > 1 { 2 } else { 1 })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_label_from_convention() {
        let label = resolve_label("proj", "0", "proj__cc_1", "some title");
        assert_eq!(label, "cc_1");
    }

    #[test]
    fn resolve_label_from_pane_title() {
        let label = resolve_label("proj", "0", "zsh", "proj__gmi_2_opus");
        assert_eq!(label, "gmi_2");
    }

    #[test]
    fn resolve_label_fallback() {
        let label = resolve_label("myproject", "3", "zsh", "just a title");
        assert_eq!(label, "myproject:3");
    }

    #[test]
    fn pill_contains_catppuccin_structure() {
        let pill = format_pill("💬", "cc_1");
        // Has left separator
        assert!(pill.contains("\u{e0b6}"));
        // Has right separator
        assert!(pill.contains("\u{e0b4}"));
        // Has accent color
        assert!(pill.contains(THM_MAUVE));
        // Has surface_0 for text bg
        assert!(pill.contains(THM_SURFACE_0));
        // Has crust for icon fg
        assert!(pill.contains(THM_CRUST));
        // Has the label
        assert!(pill.contains("cc_1"));
    }

    #[test]
    fn format_single_entry() {
        let entries = vec![AlertEntry {
            icon: "💬".to_string(),
            label: "cc_1".to_string(),
        }];
        let result = format_entries(&entries, 60);
        assert!(result.contains("💬"));
        assert!(result.contains("cc_1"));
    }

    #[test]
    fn format_multiple_entries() {
        let entries = vec![
            AlertEntry {
                icon: "💬".to_string(),
                label: "cc_1".to_string(),
            },
            AlertEntry {
                icon: "✅".to_string(),
                label: "cc_2".to_string(),
            },
        ];
        let result = format_entries(&entries, 60);
        assert!(result.contains("cc_1"));
        assert!(result.contains("cc_2"));
    }

    #[test]
    fn empty_when_no_entries() {
        let result = format_entries(&[], 60);
        assert!(result.is_empty());
    }

    #[test]
    fn strip_styles() {
        let input = "#[fg=#cba6f7]💬 cc_1#[fg=default]";
        let stripped = strip_tmux_styles(input);
        assert_eq!(stripped, "💬 cc_1");
    }

    #[test]
    fn truncation_with_overflow_pill() {
        let entries: Vec<AlertEntry> = (0..10)
            .map(|i| AlertEntry {
                icon: "💬".to_string(),
                label: format!("cc_{}", i),
            })
            .collect();
        let result = format_entries(&entries, 30);
        assert!(result.contains("+"));
    }
}
