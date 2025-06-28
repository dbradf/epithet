use std::{
    collections::HashMap,
    fs,
    process::{exit, Command, ExitStatus},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use toml::Table;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EpithetConfig {
    pub global_expansions: Option<HashMap<String, String>>,

    #[serde(flatten)]
    pub aliases: Option<HashMap<String, Alias>>,
}

impl EpithetConfig {
    pub fn read(path: &str) -> Result<Self> {
        let config_contents = fs::read_to_string(path)?;

        Ok(toml::from_str(&config_contents)?)
    }

    pub fn execute(&self, alias: &str, args: &str) -> Result<()> {
        if let Some(alias_list) = &self.aliases {
            if let Some(alias) = alias_list.get(alias) {
                let global_expansions = self.global_expansions.clone().unwrap_or_default();
                alias.execute(args, &global_expansions)?;
            } else {
                anyhow::bail!("Alias not found: {}", alias);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Alias {
    #[serde(flatten)]
    pub command: Option<Execution>,
    pub sub_aliases: Option<Vec<SubAlias>>,
    pub expansions: Option<HashMap<String, String>>,
}

impl Alias {
    pub fn execute(&self, args: &str, global_expansions: &HashMap<String, String>) -> Result<()> {
        if let Some(command) = &self.command {
            return command.execute(args, &self.get_expansions(global_expansions));
        }

        let split_arguments = args.split_whitespace().collect::<Vec<&str>>();
        let sub_command = split_arguments.first().expect("No sub command provided");
        let rest = split_arguments[1..].join(" ");

        if let Some(sub_aliases) = &self.sub_aliases {
            for sub_alias in sub_aliases {
                if sub_alias.name == *sub_command {
                    return sub_alias
                        .execution
                        .execute(&rest, &self.get_expansions(global_expansions));
                }
            }
        }

        Ok(())
    }

    fn get_expansions(
        &self,
        global_expansions: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut expansions = global_expansions.clone();

        if let Some(sub_expansions) = &self.expansions {
            for (key, value) in sub_expansions {
                expansions.insert(key.clone(), value.clone());
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
    pub fn execute(&self, args: &str, expansions: &HashMap<String, String>) -> Result<()> {
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
            Execution::Pipeline(items) => todo!(),
        }
    }

    fn get_arguments(
        &self,
        command: &str,
        arguments: &str,
        expansions: &HashMap<String, String>,
    ) -> Vec<String> {
        let tokens = self.tokenize_arguments(command, arguments);
        tokens
            .into_iter()
            .map(|token| {
                dbg!(&token);
                dbg!(&expansions);
                if token.starts_with("@") {
                    let key = token.trim_start_matches("@").to_string();
                    expansions.get(&key).unwrap_or(&token).to_string()
                } else {
                    token
                }
            })
            .collect()
    }

    fn tokenize_arguments(&self, command: &str, arguments: &str) -> Vec<String> {
        let mut tokens = tokenize_string(command);
        tokens.extend(tokenize_string(arguments));
        tokens
    }
}

fn execute_command(tokens: &[String]) -> Result<ExitStatus> {
    let cmd = tokens.first().expect("No command provided");

    dbg!(&tokens);

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
