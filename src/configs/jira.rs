// This file is part of Lectev.
//
//  Lectev is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  Lectev is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with Lectev.  If not, see <https://www.gnu.org/licenses/>.

//! Provides configuration for Jira commands
//!
//! This module provides for configuration of the system using serde structs and
//! yaml
use crate::config;
use crate::lib::jira::core::{ItemStatus, Resolution};
use crate::lib::jira::native::CustomFieldName;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use url::Url;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not open config from {}: {}", filename.display(), source))]
    OpenConfig {
        filename: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not parse config from {}: {}", filename.display(), source))]
    ParseYaml {
        filename: PathBuf,
        source: serde_yaml::Error,
    },
    #[snafu(display("Couldn't get config dir: {}", source))]
    CouldntGetConfigDir { source: config::Error },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueTypes {
    pub features: Vec<String>,
    pub operational: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub jira_instance: Url,
    pub username: String,
    pub token: String,
    pub resolution_field: Option<CustomFieldName>,
    pub issue_types: IssueTypes,
    pub status_mapping: HashMap<String, ItemStatus>,
    pub resolution_mapping: HashMap<String, Resolution>,
}

pub async fn resolve_config_path(config_path: &Option<PathBuf>) -> Result<PathBuf, Error> {
    match config_path {
        Some(resolved_config_path) => Ok(resolved_config_path.clone()),
        None => {
            let mut resolved_config_path = config::dir().await.context(CouldntGetConfigDir {})?;

            resolved_config_path.push("jira");
            resolved_config_path.set_extension("yml");
            Ok(resolved_config_path)
        }
    }
}

pub async fn read(opt_config_path: &Option<PathBuf>) -> Result<Config, Error> {
    let path = resolve_config_path(opt_config_path).await?;

    let contents = fs::read_to_string(path.clone()).await.context(OpenConfig {
        filename: path.clone(),
    })?;
    let config = serde_yaml::from_str(&contents).context(ParseYaml { filename: path })?;

    Ok(config)
}
