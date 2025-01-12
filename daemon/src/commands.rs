use std::collections::HashMap;
use std::process::Command;

pub fn execute(
    command: &str,
    shell: &str,
    actions: &HashMap<String, String>,
    commands: &HashMap<String, String>,
) -> String {
    if let Some((action_type, arg)) = parse_command(command) {
        if let Some(base_command) = actions.get(action_type) {
            if let Some(action_arg) = commands.get(arg) {
                let full_command = format!("{} {}", base_command, action_arg);

                match Command::new(shell)
                    .arg("-c")
                    .arg(&full_command)
                    .spawn()
                {
                    Ok(_) => {
                        println!("Executing in shell: {} {}", shell, full_command);
                        format!("Action '{}' with argument '{}' executed successfully!", action_type, action_arg)
                    }
                    Err(e) => {
                        eprintln!("Error executing in shell: {} {}: {:?}", shell, full_command, e);
                        format!("Failed to execute action '{}': {:?}", command, e)
                    }
                }
            } else {
                format!("Command argument '{}' not found in [commands] configuration.", arg)
            }
        } else {
            format!("Action '{}' not defined in [actions]", action_type)
        }
    } else {
        eprintln!("Invalid command format received: '{}'", command);
        format!("Invalid command format: '{}'", command)
    }
}

fn parse_command(command: &str) -> Option<(&str, &str)> {
    if let Some(start) = command.find('[') {
        if let Some(end) = command.find(']') {
            let action_type = &command[start + 1..end];
            let argument = command[end + 1..].trim_start_matches(&[' ', '\t'][..]);
            if !action_type.is_empty() && !argument.is_empty() {
                return Some((action_type, argument));
            }
        }
    }
    None
}