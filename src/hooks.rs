use crate::config::{Config, Input, Session, TEMP_SESSION_NAME};
use anyhow::Result;

pub fn before_exit(session: &mut Session) {
    if session.user_messages_len() == 0 {
        // println!("No messages in session, not saving");
        session.set_save_session(Some(false));
    }
}

pub fn before_chat_completion(config: &mut Config, input: &Input) -> Result<()> {
    let new_prompt = fill_role_template(&config)?;

    if let Some(session) = &mut config.session {
        if let Some(msg) = session.messages.first_mut() {
            msg.content = crate::client::MessageContent::Text(new_prompt);
        } else {
            // println!("No messages in session, adding new prompt");
            session.messages.push(crate::client::Message::new(
                crate::client::MessageRole::System,
                crate::client::MessageContent::Text(new_prompt),
            ));
        }

        if session.name() == TEMP_SESSION_NAME
            && session.save_session() == Some(true)
            && session.autoname() == None
        {
            if session.user_messages_len() > 1 {
                let messages = session.build_messages(input);
                let max_history_len = 140;

                let mut history = String::new();
                for message in messages {
                    let mut entry = message.content.to_text();
                    if entry.len() > max_history_len {
                        entry.truncate(max_history_len);
                        entry.push_str("[...]");
                    }

                    if message.role.is_system() {
                        continue;
                    }

                    if message.role.is_user() {
                        history.push_str("User: ");
                    } else {
                        history.push_str("Assistant: ");
                    }

                    history.push_str("\n");
                    history.push_str(&entry);
                    history.push_str("\n\n");
                }
                session.set_autoname_from_chat_history(history);
            }
        }
    }

    Ok(())
}

fn fill_role_template(config: &Config) -> Result<String> {
    let role = config.extract_role();
    let prompt = role.prompt();
    let name = role.name();

    let roles_dir = Config::roles_dir();
    let roles_dir_str = roles_dir.to_string_lossy();
    let role_env_file = format!("{}/{}.sh", roles_dir_str, name);
    let common_env_file = "~/.config/aichat/roles/env.sh";

    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            r#"
source {}
source {} 2>/dev/null || true
envsubst <<'EOF'
{}
EOF"#,
            common_env_file, role_env_file, prompt
        ))
        .output()?;

    Ok(String::from_utf8(output.stdout)?)
}
