use cron::Schedule;
use serde::{Deserialize, Deserializer};
use std::error::Error;
use std::path::Path;
use std::str::FromStr;

fn deserialize_schedule<'de, D>(deserializer: D) -> Result<Schedule, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    Schedule::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct Task {
    pub name: String,

    #[serde(deserialize_with = "deserialize_schedule")]
    pub cron_schedule: Schedule,

    pub command: String,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub tasks: Vec<Task>,
}

pub fn load_config(path: &Path) -> Result<Config, Box<dyn Error>> {
    use std::fs;

    let content = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&content)?;

    Ok(config)
}
