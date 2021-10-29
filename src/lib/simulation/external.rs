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

//! # Data types related to simulation
//!
//! This module provides the data types associated with a simulation
use chrono::NaiveDateTime;
use derive_more::Display;
use serde::{de::Error as DeR, Deserialize, Deserializer, Serialize};
use snafu::Snafu;
use std::collections::HashSet;

#[derive(Debug, Snafu)]
pub enum Error {
    /// Empty string provided for worker id
    #[snafu(display("Worker id can not be an empty string: {}", id))]
    InstantiateWorkerId { id: String },
    /// Empty string provided for work item id
    #[snafu(display("Work item id can not be an empty string: {}", id))]
    CreateWorkItemId { id: String },
    /// Empty string provided for work group id
    #[snafu(display("Work group id can not be an empty string: {}", id))]
    CreateWorkGroupId { id: String },
    /// Empty string provided for creating a skill
    #[snafu(display("Skill can not be an empty string: {}", id))]
    InstantiateSkill { id: String },
}

#[derive(Display, Debug, Serialize, Hash, PartialEq, PartialOrd)]
pub struct WorkerId(String);

impl WorkerId {
    fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::InstantiateWorkerId { id: value })
        } else {
            Ok(Self(value))
        }
    }
}

impl<'de> Deserialize<'de> for WorkerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Deserialize::deserialize(deserializer)?;
        WorkerId::new(v).map_err(D::Error::custom)
    }
}

#[derive(Display, Debug, Serialize, Hash, Eq, PartialEq, PartialOrd)]
pub struct WorkItemId(String);

impl WorkItemId {
    fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::CreateWorkItemId { id: value })
        } else {
            Ok(Self(value))
        }
    }
}

impl<'de> Deserialize<'de> for WorkItemId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Deserialize::deserialize(deserializer)?;
        WorkItemId::new(v).map_err(D::Error::custom)
    }
}

#[derive(Display, Debug, Serialize, Hash, PartialEq, Eq, PartialOrd)]
pub struct WorkGroupId(String);

impl WorkGroupId {
    fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::CreateWorkGroupId { id: value })
        } else {
            Ok(Self(value))
        }
    }
}

impl<'de> Deserialize<'de> for WorkGroupId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Deserialize::deserialize(deserializer)?;
        WorkGroupId::new(v).map_err(D::Error::custom)
    }
}

#[derive(Display, Debug, Serialize, Hash, PartialEq, Eq, PartialOrd)]
pub struct Skill(String);

impl Skill {
    fn new(value: String) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::InstantiateSkill { id: value })
        } else {
            Ok(Self(value))
        }
    }
}

impl<'de> Deserialize<'de> for Skill {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Deserialize::deserialize(deserializer)?;
        Skill::new(v).map_err(D::Error::custom)
    }
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(fmt = "Pto {{date: {}, percentage: {}}}", date, percentage)]
pub struct Pto {
    pub date: NaiveDateTime,
    pub percentage: percentage_rs::Percentage,
}

/// Represents an individual doing work in the system. Each individual has a set of skills. Those
/// skills map to the skills required to do a unit of work.
#[derive(Display, Debug, Serialize, Deserialize)]
#[display(fmt = "Worker {{id: {}, skills: {:?}, pto: {}}}", id, skills, pto)]
pub struct Worker {
    pub id: WorkerId,
    pub skills: HashSet<Skill>,
    pub pto: Pto,
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(fmt = "Estimate {{id: {}, p5: {}, p95: {}}}", id, p5, p95)]
pub struct Estimate {
    pub id: WorkerId,
    pub p5: f32,
    pub p95: f32,
}

#[derive(Display, Debug, Serialize, Deserialize, Hash, PartialOrd, PartialEq)]
pub enum WorkItemOrGroupId {
    WorkItem(WorkItemId),
    WorkGroup(WorkGroupId),
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(
    fmt = "WorkItem {{id: {}, estimates: {:?}, dependencies: {:?}, skills: {:?}}}",
    id,
    estimates,
    dependencies,
    skills
)]
pub struct WorkItem {
    pub id: WorkItemId,
    pub estimates: Vec<(WorkerId, Estimate)>,
    pub dependencies: Vec<WorkItemOrGroupId>,
    pub skills: HashSet<Skill>,
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(fmt = "WorkGroup {{id: {}, children: {:?}}}", id, children)]
pub struct WorkGroup {
    pub id: WorkGroupId,
    pub children: Vec<Work>,
    pub dependencies: Vec<WorkItemOrGroupId>,
}

#[derive(Display, Debug, Serialize, Deserialize)]
pub enum Work {
    WorkGroup(WorkGroup),
    WorkItem(WorkItem),
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(fmt = "Simulation {{workers: {:?}, work: {:?}}}", workers, work)]
pub struct Simulation {
    pub workers: Vec<Worker>,
    pub work: Vec<Work>,
}

#[derive(Display, Debug, Serialize, Deserialize)]
#[display(
    fmt = "Projection {{item: {}, projected_completion_date: {}}}",
    item,
    projected_completion_date
)]
pub struct Projection {
    pub item: WorkItemOrGroupId,
    pub projected_completion_date: NaiveDateTime,
}
