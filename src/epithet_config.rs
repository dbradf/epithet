use std::{
    collections::HashMap,
    fs,
    process::{exit, Command},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use toml::Table;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EpithetConfig {
    pub global_expansions: Option<Table>,

    #[serde(flatten)]
    pub aliases: Option<HashMap<String, Alias>>,
}

impl EpithetConfig {
    pub fn read(path: &str) -> Result<Self> {
        let config_contents = fs::read_to_string(path)?;

        Ok(toml::from_str(&config_contents)?)
    }

    pub fn execute(&self, alias: &str) -> Result<()> {
        if let Some(alias_list) = &self.aliases {
            if let Some(alias) = alias_list.get(alias) {
                alias.execute()?;
            }
        }

        Ok(())
    }

    pub fn fake() -> Self {
        Self {
            global_expansions: None,
            aliases: Some(HashMap::from([
                (
                    "t".to_string(),
                    Alias {
                        command: Some(Execution::Command("echo hello world".to_string())),
                        sub_aliases: None,
                    },
                ),
                (
                    "y".to_string(),
                    Alias {
                        command: Some(Execution::Command("yarn".to_string())),
                        sub_aliases: None,
                    },
                ),
                (
                    "a".to_string(),
                    Alias {
                        command: None,
                        sub_aliases: Some(vec![
                            SubAlias {
                                name: "test".to_string(),
                                execution: Execution::Command("yarn app".to_string()),
                            },
                            SubAlias {
                                name: "b1".to_string(),
                                execution: Execution::And(vec![
                                    "yarn workspace {0} build".to_string()
                                ]),
                            },
                        ]),
                    },
                ),
            ])),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Alias {
    #[serde(flatten)]
    pub command: Option<Execution>,
    pub sub_aliases: Option<Vec<SubAlias>>,
}

impl Alias {
    pub fn execute(&self) -> Result<()> {
        if let Some(command) = &self.command {
            command.execute()?;
        }

        Ok(())
    }
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
    pub fn execute(&self) -> Result<()> {
        match self {
            Execution::Command(command) => {
                dbg!(&command);
                let cmd = command.split(" ").collect::<Vec<&str>>();

                let result = Command::new(cmd[0])
                    .args(&cmd[1..])
                    .status()
                    .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;
                if !result.success() {
                    exit(result.code().unwrap_or(1));
                }
                Ok(())
            }
            Execution::And(items) => todo!(),
            Execution::Or(items) => todo!(),
            Execution::Pipeline(items) => todo!(),
        }
    }
}
