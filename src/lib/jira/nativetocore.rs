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
//! # Jira to Core Translation
//!
//! This module exists to translate from the internal jira format to the core format of the
//! system. It should *not* be doing io or any other side effecty thing. It only exists to do that
//! translation. If more data is needed or needed in a different way then the api should be
//! modified.
//!
//! This is simply a A -> B translation.
use crate::configs::jira;
use crate::lib::jira::native;
use crate::lib::jira::{api, core};
use chrono::{DateTime, Utc};
use snafu::{Backtrace, ResultExt, Snafu};
use std::str::FromStr;
use uom::si::f64::Time;
use uom::si::time::second;
use url::ParseError;
use uuid::Uuid;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("No mapping for resolution {}", unmapped_resolution_name))]
    MissingResolutionMapping {
        unmapped_resolution_name: String,
        backtrace: Backtrace,
    },
    #[snafu(display("No mapping for status {}", unmapped_status_name))]
    MissingStatusMapping {
        unmapped_status_name: String,
        backtrace: Backtrace,
    },
    #[snafu(display(
        "Invalid resolution field could not extract value from {} in issue {}",
        resolution_field,
        issue_key
    ))]
    InvalidResolutionField {
        resolution_field: String,
        issue_key: String,
        backtrace: Backtrace,
    },
    #[snafu(display("Could not create new url for {}: {}", target, source))]
    CouldNotCreateUrl { target: String, source: ParseError },
    #[snafu(display("Can not close closed status"))]
    CanNotCloseClosedStatus {},
    #[snafu(display("Can not close estimate"))]
    CanNotCloseEstimate {},
    #[snafu(display("Unable to parse field ({}) into days: {}", value, source))]
    UnableToParseDays {
        value: String,
        source: std::num::ParseFloatError,
    },
}

fn get_status_mapping(
    conf: &jira::Config,
    jira_status_name: &str,
) -> Result<core::ItemStatus, Error> {
    match conf.status_mapping.get(jira_status_name) {
        Some(item_status) => Ok(item_status.clone()),
        None => MissingStatusMapping {
            unmapped_status_name: jira_status_name.to_owned(),
        }
        .fail(),
    }
}

#[derive(Clone)]
struct EntryMarker {
    completed_entry: core::ItemTimeLineEntry,
    new_entry: core::ItemTimeLineEntry,
}

fn close_entry(
    old_entry: &core::ItemTimeLineEntry,
    end_date: &DateTime<Utc>,
) -> Result<core::ItemTimeLineEntry, Error> {
    match old_entry {
        core::ItemTimeLineEntry::OpenStatus {
            start: start_date,
            status,
        } => Ok(core::ItemTimeLineEntry::ClosedStatus {
            status: status.clone(),
            start: *start_date,
            end: *end_date,
        }),
        core::ItemTimeLineEntry::ClosedStatus { .. } => CanNotCloseClosedStatus.fail(),
        core::ItemTimeLineEntry::Estimate { .. } => CanNotCloseEstimate.fail(),
    }
}

fn handle_changelog_entry<'a>(
    conf: &jira::Config,
    open_entry: &'a core::ItemTimeLineEntry,
    new_start_date: &'a DateTime<Utc>,
    entry: &native::ChangeLogEntry,
) -> Result<Option<EntryMarker>, Error> {
    match (&entry.to_string, entry.field.as_str()) {
        (Some(name), "status") => {
            let new_status = get_status_mapping(conf, name)?;
            let started_entry = core::ItemTimeLineEntry::OpenStatus {
                start: *new_start_date,
                status: new_status,
            };
            let entry = close_entry(open_entry, new_start_date)?;
            Ok(Some(EntryMarker {
                completed_entry: entry,
                new_entry: started_entry,
            }))
        }
        (_, "timeestimate") => {
            if let Some(estimate_string) = &entry.to {
                let entry = core::ItemTimeLineEntry::Estimate {
                    start: *new_start_date,
                    days: Time::new::<second>(f64::from_str(estimate_string).context(
                        UnableToParseDays {
                            value: estimate_string.clone(),
                        },
                    )?),
                };
                Ok(Some(EntryMarker {
                    completed_entry: entry,
                    new_entry: (*open_entry).clone(),
                }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn convert_changelog(
    conf: &jira::Config,
    issue: &native::Issue,
    changelog: &[native::ChangeGroup],
) -> Result<Vec<core::ItemTimeLineEntry>, Error> {
    let mut last_status = core::ItemTimeLineEntry::OpenStatus {
        start: issue.fields.created,
        status: core::ItemStatus::ToDo,
    };

    let mut item_change_log = Vec::new();
    for group in changelog {
        for entry in &group.items {
            if let Some(EntryMarker {
                completed_entry,
                new_entry,
            }) = handle_changelog_entry(conf, &last_status, &group.created, entry)?
            {
                item_change_log.push(completed_entry);
                last_status = new_entry;
            }
        }
    }

    item_change_log.push(last_status);

    Ok(item_change_log)
}

fn get_resolution_value_mapping(
    conf: &jira::Config,
    jira_resolution_name: &str,
) -> Result<core::Resolution, Error> {
    match conf.resolution_mapping.get(jira_resolution_name) {
        Some(resolution) => Ok(resolution.clone()),
        None => MissingResolutionMapping {
            unmapped_resolution_name: jira_resolution_name.to_owned(),
        }
        .fail(),
    }
}

fn extract_value_from_field(
    conf: &jira::Config,
    issue_key: &native::IssueKey,
    value: &serde_json::Map<String, serde_json::Value>,
) -> Result<core::Resolution, Error> {
    match value.get("value") {
        Some(serde_json::Value::String(name)) => get_resolution_value_mapping(conf, name),
        Some(_) | None => InvalidResolutionField {
            resolution_field: conf
                .resolution_field
                .as_ref()
                .map_or_else(|| "".to_owned(), |field| field.0.clone()),
            issue_key: issue_key.0.clone(),
        }
        .fail(),
    }
}

fn get_custom_resolution_with_mapping(
    conf: &jira::Config,
    resolution_field: &native::CustomFieldName,
    issue: &native::Issue,
) -> Result<core::Resolution, Error> {
    match issue.fields.custom_fields.get(resolution_field) {
        Some(serde_json::Value::Object(value_map)) => {
            extract_value_from_field(conf, &issue.key, value_map)
        }
        Some(serde_json::Value::Null) | None => Ok(core::Resolution::UnResolved),
        Some(_) => InvalidResolutionField {
            resolution_field: conf
                .resolution_field
                .as_ref()
                .map_or_else(|| "".to_owned(), |field| field.0.clone()),
            issue_key: issue.key.0.clone(),
        }
        .fail(),
    }
}

fn get_resolution_with_mapping(
    conf: &jira::Config,
    issue: &native::Issue,
) -> Result<core::Resolution, Error> {
    match &issue.fields.resolution {
        Some(resolution) => get_resolution_value_mapping(conf, &resolution.name),
        None => Ok(core::Resolution::UnResolved),
    }
}

fn get_resolution(conf: &jira::Config, issue: &native::Issue) -> Result<core::Resolution, Error> {
    match &conf.resolution_field {
        Some(resolution_name) => get_custom_resolution_with_mapping(conf, resolution_name, issue),
        None => get_resolution_with_mapping(conf, issue),
    }
}

fn convert_issue_type(
    conf: &jira::Config,
    issue_type: &native::IssueType,
) -> Option<core::ItemType> {
    let issue_type_name = issue_type.name.as_str();
    if conf
        .issue_types
        .features
        .iter()
        .any(|member| member == issue_type_name)
    {
        Some(core::ItemType::Feature)
    } else if conf
        .issue_types
        .operational
        .iter()
        .any(|member| member == issue_type_name)
    {
        Some(core::ItemType::Operational)
    } else {
        None
    }
}

fn convert_issue(
    conf: &jira::Config,
    issue_detail: &api::IssueDetail,
) -> Result<Option<core::Item>, Error> {
    let id = core::ItemId(Uuid::new_v4());
    let description = issue_detail.issue.fields.summary.clone();
    let native_url = issue_detail
        .issue
        .sel
        .join(&format!("/browse/{}", issue_detail.issue.key))
        .context(CouldNotCreateUrl { target: "issue" })?;
    let native_id = core::NativeId(issue_detail.issue.key.0.clone());
    let timeline = convert_changelog(conf, &issue_detail.issue, &issue_detail.changelog)?;
    let current_status = get_status_mapping(conf, &issue_detail.issue.fields.status.name)?;
    let resolution = get_resolution(conf, &issue_detail.issue)?;
    match convert_issue_type(conf, &issue_detail.issue.fields.issuetype) {
        Some(issue_type) => Ok(Some(core::Item {
            id,
            name: issue_detail.issue.key.0.clone(),
            native_id,
            native_url,
            typ: issue_type,
            description,
            timeline,
            status: current_status,
            resolution,
        })),
        None => Ok(None),
    }
}

pub fn translate(
    conf: &jira::Config,
    issues: &[api::IssueDetail],
) -> Result<Vec<core::Item>, Error> {
    let mut items: Vec<core::Item> = Vec::with_capacity(issues.len());

    for issue in issues {
        if let Some(item) = convert_issue(conf, issue)? {
            items.push(item);
        }
    }

    Ok(items)
}
