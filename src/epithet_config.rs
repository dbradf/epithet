use core::fmt;
use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::{exit, Command, ExitStatus},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

const BASE_NAME: &str = "epithet";
const CONFIG_NAME: &str = "epithet.toml";

pub fn get_config_path() -> PathBuf {
    dirs::config_local_dir()
        .unwrap_or(dirs::home_dir().unwrap().join(".config"))
        .join(BASE_NAME)
        .join(CONFIG_NAME)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EpithetConfig {
    pub global_expansions: Option<HashMap<String, String>>,

    #[serde(flatten)]
    pub aliases: Option<HashMap<String, Alias>>,
}

impl EpithetConfig {
    pub fn new() -> Result<Self> {
        let config_path = get_config_path();

        Self::read(&config_path)
    }

    fn read(path: &Path) -> Result<Self> {
        let config_contents = fs::read_to_string(path)?;

        Ok(toml::from_str(&config_contents)?)
    }

    pub fn lookup_alias(&self, alias: &str, args: &[String]) -> Option<String> {
        if let Some(alias) = self.find_alias(alias) {
            return alias.lookup(args);
        }

        None
    }

    fn find_alias(&self, alias: &str) -> Option<&Alias> {
        if let Some(alias_list) = &self.aliases {
            if let Some(alias) = alias_list.get(alias) {
                return Some(alias);
            }
        }

        None
    }

    pub fn execute(&self, alias: &str, args: &[String]) -> Result<()> {
        if let Some(alias) = self.find_alias(alias) {
            let global_expansions = self.global_expansions.clone().unwrap_or_default();
            alias.execute(args, &global_expansions)?;
        } else {
            anyhow::bail!("Alias not found: {}", alias);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Alias {
    #[serde(flatten)]
    pub command: Option<Execution>,
    pub sub_aliases: Option<Vec<SubAlias>>,
    pub expansions: Option<Vec<Expansion>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Expansion {
    pub key: String,
    pub value: String,
}

impl Alias {
    pub fn execute(
        &self,
        args: &[String],
        global_expansions: &HashMap<String, String>,
    ) -> Result<()> {
        if let Some(sub_command) = args.first() {
            if let Some(sub_aliases) = &self.sub_aliases {
                for sub_alias in sub_aliases {
                    if sub_alias.name == *sub_command {
                        let rest = &args[1..];
                        return sub_alias
                            .execution
                            .execute(rest, &self.get_expansions(global_expansions));
                    }
                }
            }
        }

        if let Some(command) = &self.command {
            return command.execute(args, &self.get_expansions(global_expansions));
        }

        Ok(())
    }

    pub fn lookup(&self, args: &[String]) -> Option<String> {
        if let Some(sub_command) = args.first() {
            if let Some(sub_aliases) = &self.sub_aliases {
                for sub_alias in sub_aliases {
                    if sub_alias.name == *sub_command {
                        return Some(format!("{}", sub_alias.execution));
                    }
                }
            }
        }

        if let Some(command) = &self.command {
            return Some(format!("{}", command));
        }

        None
    }

    fn get_expansions(
        &self,
        global_expansions: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut expansions = global_expansions.clone();

        if let Some(sub_expansions) = &self.expansions {
            for expansion in sub_expansions {
                expansions.insert(expansion.key.clone(), expansion.value.clone());
            }
        }

        expansions
    }
}

fn tokenize_string(string: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_quotes = false;
    let mut in_escape = false;

    string.chars().for_each(|ch| match ch {
        '\\' if !in_escape => {
            in_escape = true;
        }
        '"' if !in_escape => {
            in_quotes = !in_quotes;
        }
        ch if ch.is_whitespace() && !in_quotes && !in_escape => {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }
        }
        _ => {
            current_token.push(ch);
            in_escape = false;
        }
    });

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubAlias {
    pub name: String,

    #[serde(flatten)]
    pub execution: Execution,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    Command(String),
    And(Vec<String>),
    Or(Vec<String>),
    Pipeline(Vec<String>),
}

impl Execution {
    pub fn execute(&self, args: &[String], expansions: &HashMap<String, String>) -> Result<()> {
        match self {
            Execution::Command(command) => {
                let tokens = self.get_arguments(command, args, expansions);
                let result = execute_command(&tokens)?;
                if !result.success() {
                    exit(result.code().unwrap_or(1));
                }
                Ok(())
            }
            Execution::And(items) => {
                for item in items {
                    let tokens = self.get_arguments(item, args, expansions);
                    let result = execute_command(&tokens)?;
                    if !result.success() {
                        exit(result.code().unwrap_or(1));
                    }
                }
                Ok(())
            }
            Execution::Or(items) => {
                let mut last_result = None;
                for item in items {
                    let tokens = self.get_arguments(item, args, expansions);
                    let result = execute_command(&tokens)?;
                    if result.success() {
                        return Ok(());
                    }
                    last_result = Some(result);
                }
                exit(
                    last_result
                        .map(|r| r.code())
                        .unwrap_or(Some(1))
                        .unwrap_or(1),
                );
            }
            Execution::Pipeline(_items) => todo!(),
        }
    }

    fn get_arguments(
        &self,
        command: &str,
        arguments: &[String],
        expansions: &HashMap<String, String>,
    ) -> Vec<String> {
        let argument_tokens: Vec<String> = arguments
            .iter()
            .flat_map(|arg| {
                if arg.starts_with("@") {
                    let key = arg.trim_start_matches("@").to_string();
                    let value = expansions.get(&key).unwrap_or(arg).to_string();
                    tokenize_string(&value)
                } else {
                    vec![arg.to_string()]
                }
            })
            .collect();

        self.expand_command(command, &argument_tokens)
    }

    fn expand_command(&self, command: &str, arguments: &[String]) -> Vec<String> {
        let mut arguments_copy: Vec<Option<String>> =
            arguments.iter().map(|a| Some(a.to_string())).collect();
        let command_tokens = tokenize_string(command);

        let mut tokens: Vec<String> = command_tokens
            .into_iter()
            .map(|token| {
                if token.starts_with("{") && token.ends_with("}") {
                    if let Ok(position) = &token[1..token.len() - 1].parse::<usize>() {
                        if position < &arguments.len() {
                            arguments_copy[*position] = None;
                            return arguments[*position].clone();
                        }
                    }
                }

                token.to_string()
            })
            .collect();

        tokens.extend(arguments_copy.into_iter().flatten());

        tokens
    }
}

impl Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Execution::Command(command) => write!(f, "{}", command),
            Execution::And(items) => write!(f, "{}", items.join(" && ")),
            Execution::Or(items) => write!(f, "{}", items.join(" || ")),
            Execution::Pipeline(items) => write!(f, "{}", items.join(" | ")),
        }
    }
}

fn execute_command(tokens: &[String]) -> Result<ExitStatus> {
    let cmd = shellexpand::tilde(tokens.first().expect("No command provided")).to_string();

    Command::new(cmd)
        .args(&tokens[1..])
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("echo \"Hello, world!\"", vec!["echo", "Hello, world!"])]
    #[case("echo Hello, world!", vec!["echo", "Hello,", "world!"])]
    #[case("echo \"Hello, \\\"world!\\\"\"", vec!["echo", "Hello, \"world!\""])]
    fn test_tokenize_string(#[case] input: &str, #[case] expected: Vec<&str>) {
        assert_eq!(tokenize_string(input), expected);
    }
}
