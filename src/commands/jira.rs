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
use crate::configs::jira as jira_config;
use crate::feature_flags;
use crate::lib::jira::api;
use crate::lib::jira::core;
use crate::lib::jira::nativetocore;
use crate::lib::jira::times_in_flight;
use crate::lib::rest;
use snafu::{ResultExt, Snafu};
use std::path::Path;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, instrument};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not get config: {}", source))]
    GetConfig { source: jira_config::Error },
    #[snafu(display("Could not build rest client {}", source))]
    FailedToBuildClient { source: rest::Error },
    #[snafu(display("Could not get data from jira {}", source))]
    FailedToGetData { source: api::Error },
    #[snafu(display("Failed to transform jira data to internal model {}", source))]
    FailedToTransformData { source: nativetocore::Error },
    #[snafu(display("Failed to create raw dump file {}", source))]
    FailedToCreateRawDumpFile { source: std::io::Error },
    #[snafu(display("Unable to convert internal structure to json {}", source))]
    FailedToConvertInternalStructureToJson { source: serde_json::Error },
    #[snafu(display("Unable to write file to:  {}", source))]
    FailedToWriteFile {
        path: String,
        source: std::io::Error,
    },
    #[snafu(display("Unable to write raw dump file to:  {}", source))]
    FailedToWriteRawDumpFile {
        path: String,
        source: std::io::Error,
    },
    #[snafu(display("Failed to create load file object {}", source))]
    FailedToReadFromFile { source: std::io::Error },
    #[snafu(display("Unable to convert json to internal structure {}", source))]
    FailedToConvertJsonToInternalStructure { source: serde_json::Error },
    #[snafu(display("Load from jira specified but no jira file specified"))]
    UnableToLoadFromJiraFile {},
    #[snafu(display("Failed to create csv output file {}", source))]
    FailedToCreateCSVFile { source: std::io::Error },
    #[snafu(display("Failed to write csv output to file {}", source))]
    FailedToWriteToCSVFile { source: csv_async::Error },
    #[snafu(display("Feature flag 'JIRA_TIME_IN_STATUS' is not enabled"))]
    FeatureFlagNotEnabled,
}

#[instrument]
async fn load_jira_from_file(load_file: &Path) -> Result<Vec<api::IssueDetail>, Error> {
    let contents = tokio::fs::read_to_string(load_file)
        .await
        .context(FailedToReadFromFile {})?;
    serde_json::from_str(&contents).context(FailedToConvertJsonToInternalStructure {})
}

#[instrument]
async fn write_json_file(dump_path: &Path, data: &[api::IssueDetail]) -> Result<(), Error> {
    let mut dump_file = File::create(dump_path)
        .await
        .context(FailedToCreateRawDumpFile {})?;
    dump_file
        .write_all(
            serde_json::to_string(&data)
                .context(FailedToConvertInternalStructureToJson {})?
                .as_bytes(),
        )
        .await
        .context(FailedToWriteFile {
            path: dump_path.to_string_lossy(),
        })?;

    Ok(())
}

#[instrument]
async fn gather_from_jira(
    conf: &jira_config::Config,
    should_load_from_jira_file: bool,
    jira_load_path: &Option<PathBuf>,
    jql: &str,
) -> Result<Vec<core::Item>, Error> {
    let issues = match (should_load_from_jira_file, jira_load_path) {
        (true, Some(load_path)) => load_jira_from_file(load_path).await?,
        (true, None) => return UnableToLoadFromJiraFile {}.fail(),
        _ => {
            let client = rest::new(&conf.jira_instance, &conf.username, &conf.token)
                .context(FailedToBuildClient {})?;
            api::get_issues_from_jql(&client, jql)
                .await
                .context(FailedToGetData {})?
        }
    };

    if let Some(jira_path) = jira_load_path {
        write_json_file(jira_path, &issues).await?;
    }

    let items = nativetocore::translate(conf, &issues).context(FailedToTransformData {})?;

    Ok(items)
}

#[instrument]
pub async fn write_records_to_csv(
    out_file: &Path,
    entries: &[times_in_flight::Entry<'_>],
) -> Result<(), Error> {
    let mut item_writer = csv_async::AsyncSerializer::from_writer(
        File::create(out_file)
            .await
            .context(FailedToCreateCSVFile {})?,
    );

    for entry in entries {
        item_writer
            .serialize(&entry)
            .await
            .context(FailedToWriteToCSVFile {})?;
    }

    Ok(())
}

#[instrument]
pub async fn do_time_in_status(
    config_path: &Option<PathBuf>,
    out_path: &Path,
    should_load_jira_from_file: bool,
    jira_load_path: &Option<PathBuf>,
    jql: &str,
) -> Result<(), Error> {
    if feature_flags::is_enabled(feature_flags::TimeInStatus) {
        let conf = jira_config::read(config_path).await.context(GetConfig {})?;

        let items =
            gather_from_jira(&conf, should_load_jira_from_file, jira_load_path, jql).await?;

        let resolved_data = times_in_flight::calculate(&conf.jira_instance, &items);

        write_records_to_csv(out_path, &resolved_data).await?;

        Ok(())
    } else {
        error!("This command is a WIP, you must set the feature flag to continue");
        FeatureFlagNotEnabled.fail()
    }
}
