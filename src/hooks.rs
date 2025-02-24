use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    client::ChatCompletionsData,
    config::{Input, Session},
};
use anyhow::{Context, Result};

pub fn before_exit(session: &mut Session) {
    let hook_name = "before_exit";

    match run_hook(hook_name, session, None) {
        Ok(Some(_updated_session)) => {
            // *session = updated_session;
            // Here we need to be careful so we don't loose any data
            // Also, we perhaps should create an alternative SessionView type that serializes more fields
        }
        Ok(None) => (), // No hook found, do nothing
        Err(e) => eprintln!("Error running before_exit hook: {}", e),
    }
}

pub fn after_prepare_chat_completion_data(data: &mut ChatCompletionsData, input: &Input) {
    let hook_name = "after_prepare_chat_completion_data";
    let env_vars = Some(vec![("AICHAT_HOOKS_ROLE", input.role().name().to_string())]);

    match run_hook(hook_name, data, env_vars) {
        Ok(Some(updated_data)) => *data = updated_data,
        Ok(None) => (),
        Err(e) => eprintln!(
            "Error running after_prepare_chat_completion_data hook: {}",
            e
        ),
    }
}

pub fn run_hook<T>(
    hook_name: &str,
    hook_input: &T,
    env_vars: Option<Vec<(&str, String)>>,
) -> Result<Option<T>>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    // Save original PATH
    let original_path = std::env::var("PATH").context("Failed to get PATH")?;

    // Setup hook PATH
    let config_dir = crate::config::Config::config_dir();
    let config_dir = config_dir
        .to_str()
        .context("Failed to convert config dir to string")?;
    let hooks_path = format!("{}/hooks", config_dir);
    let new_path = format!("{}:{}", hooks_path, original_path);
    std::env::set_var("PATH", &new_path);

    // Ensure cleanup of PATH on exit
    let cleanup_path = |original: &str| std::env::set_var("PATH", original);
    let cleanup_vars = |vars: &[(&str, String)]| {
        for (key, _) in vars {
            std::env::remove_var(key);
        }
    };

    // Early return if hook doesn't exist
    if which::which(hook_name).is_err() {
        cleanup_path(&original_path);
        return Ok(None);
    }

    // Set any provided environment variables
    if let Some(ref vars) = env_vars {
        for (key, value) in vars {
            std::env::set_var(key, value);
        }
    }

    // Run the command
    let mut child = Command::new(hook_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn hook")?;

    // Write input
    let json = serde_json::to_string(&hook_input).context("Failed to serialize input")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(json.as_bytes())
            .context("Failed to write to hook's stdin")?;
    }

    // Get output
    let output = child
        .wait_with_output()
        .context("Failed to get output from hook")?;

    // Cleanup environment
    if let Some(ref vars) = env_vars {
        cleanup_vars(vars);
    }
    cleanup_path(&original_path);

    // Print stderr as-is
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // Handle results
    if !output.status.success() {
        anyhow::bail!("Hook '{}' failed with non-zero status", hook_name);
    }

    match serde_json::from_slice(&output.stdout) {
        Ok(new_data) => Ok(new_data),
        Err(e) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Failed to parse {} hook output: {}. Output was: {}",
                hook_name,
                e,
                output_str
            )
        }
    }
}
