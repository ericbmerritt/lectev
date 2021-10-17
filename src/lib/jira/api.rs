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
//! # Jira Api Integration
//!
//! This module provides the integration to the jira api.
//! The design of the system is such that this should know *NOTHING* about the
//! core model. Its area of concern is just pulling data from jira and putting
//! it into a format that can be translated to the core format.
//!
//! ## Model
//!
//! The base cognitive model here is that each team has a board, each board has issues, each assue
//! has a changelog. Goals may reference items in the boards of teams, but may also reference
//! issues in other areas. So we get the teams and the issues related to those teams (via the
//! board) then we get the goals, and then we get every issue that a goal references that is not in
//! a team.
//!
//! ## A note on Resolutions
//!
//! Jira has a resolution field that isn't often used. Most of the time a custom resolution
//! field is used that has its own resolutions. We assume that a custom resolution field is
//! provided in the config, and use that to determine the resolution of the issue.

use crate::lib::jira::native;
use crate::lib::rest;
use backoff::future::retry;
use backoff::ExponentialBackoff;
use futures::future::{try_join_all, TryFutureExt};
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, Snafu};
use std::convert::TryFrom;
use tracing::{info, instrument};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("Unable to build request for path {}: {}", path, source))]
    UnableToBuildRequest { path: String, source: rest::Error },
    #[snafu(display(
        "Field {} in issue {} did not contain an Epic Link",
        field_name,
        issue_key
    ))]
    InvalidEpicLink {
        issue_key: native::IssueKey,
        field_name: native::CustomFieldName,
    },
    #[snafu(display("No custom fields for epic name using {}", readable_name))]
    NoEpicLinkField { readable_name: String },
    #[snafu(display("Could not get custom fields when attempting to get epic name"))]
    GetEpicLinkField { source: reqwest::Error },
    #[snafu(display(
        "Could not get changelog for issue {}, starting at {}, with max results {}: {}",
        issue_key,
        start_at,
        max_results,
        source
    ))]
    CouldNotGetChangeLogForIssue {
        issue_key: native::IssueKey,
        start_at: u64,
        max_results: u64,
        source: reqwest::Error,
    },
    #[snafu(display(
        "Could not get issues for jql ({}), starting_at: {}, with max_results{}: {}",
        jql,
        start_at,
        max_results,
        source
    ))]
    CouldNotGetIssuesForJQLQuery {
        jql: String,
        start_at: u64,
        max_results: u64,
        source: reqwest::Error,
    },
    #[snafu(display("Unable to size {} to u64, this should never happen: {}", size, source))]
    UnableToConvertUsizeToU64 {
        size: usize,
        source: std::num::TryFromIntError,
    },
    #[snafu(display("Could not add start_at"))]
    AddStartAt {},
    #[snafu(display("Max results add"))]
    AddMaxResults {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueDetail {
    pub issue: native::Issue,
    pub changelog: Vec<native::ChangeGroup>,
}

#[instrument(skip(client))]
async fn get_changelog_for_issue(
    client: &rest::Client,
    key: &native::IssueKey,
) -> Result<Vec<native::ChangeGroup>, Error> {
    info!("get changelog for {}", key);

    let mut done = false;
    let mut changelog = Vec::new();
    let mut start_at: u64 = 0;
    let max_results: u64 = 100;
    while !done {
        let result = retry(ExponentialBackoff::default(), || async {
            let changelog_path = format!("/rest/api/3/issue/{}/changelog", key);
            rest::get(client, &changelog_path)
                .context(UnableToBuildRequest {
                    path: changelog_path,
                })?
                .query(&[
                    ("startAt", &start_at.to_string()),
                    ("maxResults", &max_results.to_string()),
                ])
                .send()
                .await
                .context(CouldNotGetChangeLogForIssue {
                    issue_key: key.clone(),
                    start_at,
                    max_results,
                })?
                .json::<native::ChangeLog>()
                .await
                .context(CouldNotGetChangeLogForIssue {
                    issue_key: key.clone(),
                    start_at,
                    max_results,
                })
                .map_err(backoff::Error::Transient)
        })
        .await?;

        let len: u64 = u64::try_from(result.values.len()).context(UnableToConvertUsizeToU64 {
            size: result.values.len(),
        })?;
        start_at = len.checked_add(start_at).context(AddStartAt {})?;

        match result.is_last {
            Some(true) => done = true,
            Some(false) | None => done = len < max_results,
        }
        changelog.extend(result.values);
    }

    Ok(changelog)
}

#[instrument(skip(client))]
async fn get_all_changelogs(
    client: &rest::Client,
    issues: Vec<native::Issue>,
) -> Result<Vec<IssueDetail>, Error> {
    try_join_all(issues.iter().map(|issue| {
        let issue_clone = issue.clone();
        get_changelog_for_issue(client, &issue.key).and_then(|changelog| async {
            Ok(IssueDetail {
                issue: issue_clone,
                changelog,
            })
        })
    }))
    .await
}

#[instrument(skip(client))]
pub async fn get_issues_from_jql(
    client: &rest::Client,
    jql: &str,
) -> Result<Vec<IssueDetail>, Error> {
    let mut done = false;
    let mut work = Vec::new();
    let mut start_at: u64 = 0;
    let max_results: u64 = 100;
    let mut keys = Vec::new();
    while !done {
        let search_path = "/rest/api/3/search";
        let jql_result: native::Search = retry(ExponentialBackoff::default(), || async {
            rest::get(client, search_path)
                .context(UnableToBuildRequest { path: search_path })?
                .query(&[
                    ("jql", jql),
                    ("startAt", &start_at.to_string()),
                    ("maxResults", &max_results.to_string()),
                ])
                .send()
                .await
                .context(CouldNotGetIssuesForJQLQuery {
                    jql: jql.to_owned(),
                    start_at,
                    max_results,
                })?
                .json()
                .await
                .context(CouldNotGetIssuesForJQLQuery {
                    jql: jql.to_owned(),
                    start_at,
                    max_results,
                })
                .map_err(backoff::Error::Transient)
        })
        .await?;

        keys.extend(jql_result.issues.iter().map(|issue| issue.key.clone()));
        work.extend(get_all_changelogs(client, jql_result.issues).await?);
        start_at = jql_result
            .max_results
            .checked_add(start_at)
            .context(AddStartAt {})?;

        done = start_at >= jql_result.total;
    }

    Ok(work)
}
