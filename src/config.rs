extern crate toml;

use std::fs::File;
use std::io::{self, Read};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub min_reviewers_approved: u8,
    pub pr_max_age: u8,
    pub notification_timeout: u8,
    pub sleep_interval: u8,
    pub bitbucket: Bitbucket,
    pub slack: Slack,
}

#[derive(Deserialize, Debug)]
pub struct Bitbucket {
    pub uri: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct Slack {
    pub uri: String,
    pub username: String,
    pub channel: String,
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(io::Error),
    ParseError(toml::de::Error),
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        ConfigError::IoError(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        ConfigError::ParseError(error)
    }
}

pub fn parse(filename: &str) -> Result<Config, ConfigError> {
    let mut fd = File::open(filename)?;

    let mut contents = String::new();
    fd.read_to_string(&mut contents)?;

    let config: Config = toml::from_str(&contents)?;

    Ok(config)
}
