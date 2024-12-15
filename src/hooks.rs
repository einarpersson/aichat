use crate::config::{Config, Input, Role, TEMP_SESSION_NAME};
use anyhow::Result;

pub fn before_chat_completion(config: &mut Config, input: &Input) -> Result<()> {
    let role = config.extract_role();
    let new_prompt = fill_role_template(&role)?;

    if let Some(session) = &mut config.session {
        if let Some(msg) = session.messages.first_mut() {
            msg.content = crate::client::MessageContent::Text(new_prompt);
        } else {
            // add some debug logging
            println!("No messages in session, adding new prompt");
            session.messages.push(crate::client::Message::new(
                crate::client::MessageRole::System,
                crate::client::MessageContent::Text(new_prompt),
            ));
        }

        if session.name() == TEMP_SESSION_NAME && session.save_session() == Some(true) {
            if session.user_messages_len() > 1 {
                // println!("Activating autonaming...");
                let messages = session.echo_messages(input);
                session.set_autoname_from_chat_history(messages);
            }
        }
    }

    Ok(())
}

fn fill_role_template(role: &Role) -> Result<String> {
    let prompt = role.prompt();
    let name = role.name();

    // TODO: use the roles_dir from the global config
    let role_env_file = format!("~/config/aichat/roles/{}.sh", name);

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            r#"
source ~/config/aichat/roles/env.sh
source {} 2>/dev/null || true
envsubst <<'EOF'
{}
EOF"#,
            role_env_file, prompt
        ))
        .output()?;

    Ok(String::from_utf8(output.stdout)?)
}
