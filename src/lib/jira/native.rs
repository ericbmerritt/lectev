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
//! # Atlassian Jira Api Description
//!
//! This model contains the types that represent the jira api. These are 'discovered' via
//! calls to the api along with the use of <https://rusty-json.herokuapp.com/>. There is not
//! a current json-schema to rust translator and rusty-json provides the next best thing. There is
//! a lot of manual fixing so you can't just copy and past or rerun to generate. You have to copy
//! things over.
//!
//! So, while this works well we may run into issues where the api as returned by Jira does not
//! conform exactly to this spec. When that occurs we change the spec.
//!
//! This spec is targeted at the jira api version 3.

use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

#[derive(Display, Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct TeamName(pub String);

/// The name of custom fields in the system
#[derive(Clone, Display, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CustomFieldName(pub String);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldSchema {
    #[serde(rename = "type")]
    pub typ: String,
    pub system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldProject {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldScope {
    #[serde(rename = "type")]
    pub typ: String,
    pub project: CustomFieldProject,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct CustomField {
    pub id: CustomFieldName,
    pub key: Option<CustomFieldName>,
    pub name: CustomFieldName,
    pub custom: bool,
    pub orderable: bool,
    pub navigable: bool,
    pub searchable: bool,
    pub clause_names: Vec<String>,
    pub schema: Option<CustomFieldSchema>,
    pub untranslated_name: Option<CustomFieldName>,
    pub scope: Option<CustomFieldScope>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomFields(pub Vec<CustomField>);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Display)]
pub struct IssueKey(pub String);

#[derive(Display, Clone, Debug, Serialize, Deserialize)]
pub struct BoardName(pub String);

#[derive(Display, Clone, Debug, Serialize, Deserialize)]
pub struct ProjectName(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, Display)]
pub struct BoardId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub project_id: i64,
    pub display_name: String,
    pub project_name: String,
    pub project_key: String,
    pub project_type_key: String,
    #[serde(rename = "avatarURI")]
    pub avatar_uri: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: BoardId,
    #[serde(rename = "self")]
    pub sel: Url,
    pub name: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub location: Option<Location>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Boards {
    pub max_results: u64,
    pub start_at: u64,
    pub total: u64,
    pub is_last: Option<bool>,
    pub values: Vec<Board>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardIssues {
    pub max_results: u64,
    pub start_at: u64,
    pub total: u64,
    pub is_last: Option<bool>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLogEntry {
    pub field: String,
    pub fieldtype: String,
    pub field_id: Option<String>,
    pub from: Option<String>,
    pub from_string: Option<String>,
    pub to: Option<String>,
    pub to_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeGroup {
    pub id: String,
    pub author: Assignee,
    pub created: DateTime<Utc>,
    pub items: Vec<ChangeLogEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLog {
    #[serde(rename = "self")]
    pub sel: Option<String>,
    pub max_results: Option<u64>,
    pub start_at: Option<u64>,
    pub total: Option<u64>,
    pub is_last: Option<bool>,
    pub values: Vec<ChangeGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Priority {
    #[serde(rename = "self")]
    pub sel: Url,
    pub icon_url: Url,
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCategory {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: i64,
    pub key: String,
    pub color_name: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(rename = "self")]
    pub sel: Url,
    pub description: String,
    pub icon_url: String,
    pub name: String,
    pub id: String,
    pub status_category: StatusCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueType {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: String,
    pub description: String,
    pub icon_url: String,
    pub name: String,
    pub subtask: bool,
    pub avatar_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLinksType {
    pub id: String,
    pub name: String,
    pub inward: String,
    pub outward: String,
    #[serde(rename = "self")]
    pub sel: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutwardIssueField {
    pub summary: String,
    pub status: Status,
    pub priority: Priority,
    pub issuetype: IssueType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutwardIssue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub sel: Url,
    pub fields: OutwardIssueField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLink {
    pub id: String,
    #[serde(rename = "self")]
    pub sel: Url,
    #[serde(rename = "type")]
    pub typ: IssueLinksType,
    pub outward_issue: Option<OutwardIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: String,
    pub description: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCategory {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: String,
    pub description: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarUrl {
    #[serde(rename = "48x48")]
    pub f48x48: Url,
    #[serde(rename = "24x24")]
    pub f24x24: Url,
    #[serde(rename = "16x16")]
    pub f16x16: Url,
    #[serde(rename = "32x32")]
    pub f32x32: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignee {
    #[serde(rename = "self")]
    pub sel: Option<Url>,
    pub name: Option<String>,
    pub key: Option<String>,
    pub email_address: Option<String>,
    pub avatar_urls: AvatarUrl,
    pub display_name: String,
    pub active: bool,
    pub time_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub summary: String,
    pub status: Status,
    pub issue_type: Option<IssueType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub sel: Url,
    pub fields: Field,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vote {
    #[serde(rename = "self")]
    pub sel: Url,
    pub votes: i64,
    pub has_voted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub progress: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: String,
    pub key: String,
    pub name: String,
    pub project_type_key: String,
    pub avatar_urls: AvatarUrl,
    pub project_category: Option<ProjectCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watch {
    #[serde(rename = "self")]
    pub sel: Url,
    pub watch_count: i64,
    pub is_watching: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixVersion {
    #[serde(rename = "self")]
    pub sel: Url,
    pub id: String,
    pub name: String,
    pub archived: bool,
    pub released: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionPart {
    #[serde(rename = "type")]
    pub typ: String,
    pub content: Option<Vec<DescriptionPart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    String(String),
    Complex {
        version: u64,
        #[serde(rename = "type")]
        typ: String,
        content: Vec<DescriptionPart>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuesField {
    pub issuetype: IssueType,
    pub resolution: Option<Resolution>,
    pub issuelinks: Vec<IssueLink>,
    pub assignee: Option<Assignee>,
    pub subtasks: Vec<Subtask>,
    pub votes: Option<Vote>,
    pub status: Status,
    pub creator: Option<Assignee>,
    pub workratio: i64,
    pub labels: Vec<String>,
    pub reporter: Option<Assignee>,
    pub progress: Progress,
    pub project: Project,
    pub resolutiondate: Option<String>,
    pub watches: Watch,
    pub updated: String,
    pub description: Option<Description>,
    pub summary: String,
    pub priority: Option<Priority>,
    pub aggregateprogress: Progress,
    pub created: DateTime<Utc>,
    pub fix_versions: Vec<FixVersion>,
    #[serde(flatten)]
    pub custom_fields: HashMap<CustomFieldName, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub expand: Option<String>,
    pub id: String,
    #[serde(rename = "self")]
    pub sel: Url,
    pub key: IssueKey,
    pub fields: IssuesField,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    #[serde(rename = "self")]
    pub sel: Option<String>,
    pub max_results: u64,
    pub start_at: u64,
    pub total: u64,
    pub is_last: Option<bool>,
    pub issues: Vec<Issue>,
}
