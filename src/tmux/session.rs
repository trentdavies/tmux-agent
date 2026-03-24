use serde::Serialize;

use super::client::TmuxClient;
use super::pane::{detect_agent_from_command, parse_pane_title, Pane};
use crate::error::TaError;

const SEP: &str = "_TA_SEP_";

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub name: String,
    pub directory: String,
    pub windows: u32,
    pub attached: bool,
    pub created: String,
    pub panes: Vec<Pane>,
}

/// List all tmux sessions (without pane details).
pub async fn list_sessions(client: &TmuxClient) -> Result<Vec<Session>, TaError> {
    let format = [
        "#{session_name}",
        "#{session_path}",
        "#{session_windows}",
        "#{session_attached}",
        "#{session_created_string}",
    ]
    .join(SEP);

    let output = client.run(&["list-sessions", "-F", &format]).await?;
    let mut sessions = Vec::new();

    for line in output.lines() {
        let fields: Vec<&str> = line.split(SEP).collect();
        if fields.len() < 5 {
            continue;
        }
        sessions.push(Session {
            name: fields[0].to_string(),
            directory: fields[1].to_string(),
            windows: fields[2].parse().unwrap_or(0),
            attached: fields[3] == "1",
            created: fields[4].to_string(),
            panes: vec![],
        });
    }

    Ok(sessions)
}

/// Get a session by name, including all its panes.
pub async fn get_session(client: &TmuxClient, name: &str) -> Result<Session, TaError> {
    let sessions = list_sessions(client).await?;
    let mut session = sessions
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| TaError::SessionNotFound(name.to_string()))?;

    session.panes = list_panes(client, name).await?;
    Ok(session)
}

/// List all panes in a session.
pub async fn list_panes(client: &TmuxClient, session: &str) -> Result<Vec<Pane>, TaError> {
    let format = [
        "#{session_name}",
        "#{pane_id}",
        "#{pane_index}",
        "#{window_index}",
        "#{pane_title}",
        "#{pane_current_command}",
        "#{pane_width}",
        "#{pane_height}",
        "#{pane_active}",
        "#{pane_pid}",
        "#{pane_current_path}",
    ]
    .join(SEP);

    let output = client
        .run(&["list-panes", "-t", session, "-s", "-F", &format])
        .await?;
    Ok(parse_pane_lines(&output))
}

/// List all panes across all sessions.
pub async fn list_all_panes(client: &TmuxClient) -> Result<Vec<Pane>, TaError> {
    let format = [
        "#{session_name}",
        "#{pane_id}",
        "#{pane_index}",
        "#{window_index}",
        "#{pane_title}",
        "#{pane_current_command}",
        "#{pane_width}",
        "#{pane_height}",
        "#{pane_active}",
        "#{pane_pid}",
        "#{pane_current_path}",
    ]
    .join(SEP);

    let output = client.run(&["list-panes", "-a", "-F", &format]).await?;
    Ok(parse_pane_lines(&output))
}

fn parse_pane_lines(output: &str) -> Vec<Pane> {
    let mut panes = Vec::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split(SEP).collect();
        if fields.len() < 11 {
            continue;
        }

        let session_name = fields[0].to_string();
        let title = fields[4];
        let command = fields[5];

        let (mut agent_type, ta_index, variant, tags) = parse_pane_title(title);

        // Fallback: detect from command if title didn't match
        if agent_type == super::pane::AgentType::User && ta_index == 0 {
            let detected = detect_agent_from_command(command);
            if detected != super::pane::AgentType::User {
                agent_type = detected;
            }
        }

        panes.push(Pane {
            session_name,
            id: fields[1].to_string(),
            index: fields[2].parse().unwrap_or(0),
            window_index: fields[3].parse().unwrap_or(0),
            title: title.to_string(),
            command: command.to_string(),
            width: fields[6].parse().unwrap_or(0),
            height: fields[7].parse().unwrap_or(0),
            active: fields[8] == "1",
            pid: fields[9].parse().unwrap_or(0),
            current_path: fields[10].to_string(),
            agent_type,
            ta_index,
            variant,
            tags,
        });
    }
    panes
}
