use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    client::ChatCompletionsData,
    config::{Input, Session},
};
use anyhow::{Context, Result};

// pub fn before_exit(session: &mut Session) {
//     if session.user_messages_len() == 0 {
//         // println!("No messages in session, not saving");
//         session.set_save_session(Some(false));
//     }
// }

pub fn chat_completion_data(
    data: ChatCompletionsData,
    input: &Input,
) -> Result<ChatCompletionsData> {
    if which::which("chat_completion_data").is_err() {
        println!("chat_completion_data hook not found, skipping");
        return Ok(data);
    }

    std::env::set_var("AICHAT_HOOKS_ROLE", input.role().name());

    let mut child = Command::new("chat_completion_data")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn external command")?;

    let json = serde_json::to_string(&data).context("Failed to serialize input data")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(json.as_bytes())
            .context("Failed to write to external command")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to get output from external command")?;

    std::env::remove_var("AICHAT_HOOKS_ROLE");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("External command failed: {}", stderr);
    }

    match serde_json::from_slice(&output.stdout) {
        Ok(new_data) => Ok(new_data),
        Err(e) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Failed to parse command output: {}. Output was: {}",
                e,
                output_str
            )
        }
    }
}
