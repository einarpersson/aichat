use crate::config::{Config, Input, Role};
use anyhow::Result;

pub fn before_chat_completion(config: &mut Config, _input: &Input) -> Result<()> {
    let role = config.extract_role();
    let new_prompt = fill_role_template(&role)?;

    // Ok, remaining bugs:
    // - The role template is only filled on the second message, not from startup. Either we modify
    // the logic here and somehow inject if it is not there yet (but i'm not sure if it may have
    // other side effects), or we could try to find the place in the code where the first message
    // is generated and create a similar hook as this one (where we can call fill_role_template).
    //
    // - i've had some troubles with tool_builder role

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
