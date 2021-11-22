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
/// This modules provides a way to convert row based input like a csv file or a google sheet into
/// work items in the [`simulation::external`] format. 
use crate::lib::simulation::external as sim_external;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, Snafu};
use std::collections::HashMap;
use tracing::instrument;

#[derive(Debug, Snafu)]
pub enum Error {
    /// Error produced if a dependency can not be resolved. Dependencies must appear earlier in the
    /// template file than the item that depends on them.
    #[snafu(display("Unable to resolve dependency: {}", dep))]
    UnableToResolveDependency { dep: String },
    /// Error produced if we can't produce a [`sim_external::WorkGroupId`] from the id column of
    /// the template.
    #[snafu(display(
        "A work group id ({}) could not be created from the template data: {}",
        id,
        source
    ))]
    InvalidWorkGroupId {
        id: String,
        source: sim_external::Error,
    },
    #[snafu(display("Invalid work item on id {}", id))]
    InvalidWorkItem { id: String },
}

#[derive(Display, Debug, Deserialize, Serialize)]
#[display(
    fmt = "Display {{id: {}, rung: {:?}, task: {:?}, sub_task: {:?}, skills: {:?}, dependencies: {:?}}}",
    id,
    rung,
    task,
    sub_task,
    skills,
    dependencies
)]
pub struct Template {
    pub id: String,
    pub rung: Option<String>,
    pub task: Option<String>,
    pub sub_task: Option<String>,
    pub skills: Vec<String>,
    pub dependencies: Vec<String>,
}

enum TemplateEvent {
    ProbableWorkGroup {
        id: String,
        description: String,
        skills: Vec<String>,
        dependencies: Vec<String>,
    },
    ProbableWorkItem {
        id: String,
        description: String,
        skills: Vec<String>,
        dependencies: Vec<String>,
    },
}

/// This takes deps as strings and converts them to an id. One caveate here is that deps must
/// appear in the file top down. So this module adds the id it creates to the dep_cache as it
/// encounters them. So even if the dep appears later in the file, it will still generate an error
/// when we go to look it up.
#[instrument]
fn transform_deps(
    deps: &Vec<String>,
    dep_cache: &mut HashMap<String, sim_external::WorkItemOrGroupId>,
) -> Result<Vec<sim_external::WorkItemOrGroupId>, Error> {
    deps.iter()
        .map(|dep| {
            Ok((*dep_cache
                .get(dep)
                .context(UnableToResolveDependency { dep })?)
            .clone())
        })
        .collect::<Result<Vec<sim_external::WorkItemOrGroupId>, Error>>()
}

/// Convert a template to a Work Group. This assumes the work work has been done to ensure that the
/// template actually represents a work group.
#[instrument]
fn template_to_work_group(
    description: &str,
    template: &Template,
    dep_cache: &mut HashMap<String, sim_external::WorkItemOrGroupId>,
) -> Result<sim_external::WorkGroup, Error> {
    let dependencies = transform_deps(&template.dependencies, dep_cache)?;
    let work_group_id = sim_external::WorkGroupId::new(template.id.clone())
        .context(InvalidWorkGroupId { id: template.id })?;

    dep_cache.insert(
        template.id,
        sim_external::WorkItemOrGroupId::WorkGroup(work_group_id.clone()),
    );

    Ok(sim_external::WorkGroup {
        id: work_group_id,
        description: description.to_owned(),
        children: Vec::new(),
        dependencies,
    })
}

fn template_to_event(template: Template) -> Result<TemplateEvent, Error> {
    match (template.rung, template.task, template.sub_task) {
        (Some(rung), None, None) => Ok(TemplateEvent::ProbableWorkGroup {
            id: template.id,
            description: rung,
            skills: template.skills,
            dependencies: template.dependencies,
        }),
        (None, Some(task), None) => Ok(TemplateEvent::ProbableWorkGroup {
            id: template.id,
            description: task,
            skills: template.skills,
            dependencies: template.dependencies,
        }),
        (None, None, Some(sub_task)) => Ok(TemplateEvent::ProbableWorkItem {
            id: template.id,
            description: sub_task,
            skills: template.skills,
            dependencies: template.dependencies,
        }),
        _ => InvalidWorkItem { id: template.id }.fail(),
    }
}

/// The template is more rigid then the hierarchical work structure that we have.
/// The template has 'rungs', 'tasks' and 'sub_tasks'. Those generally equate to epics, stories
/// and sub_tasks in most peoples thinking. In our simulation structure they equate to two
/// levels of WorkGroup -> WorkGroup -> WorkItem. We allow the user to omit the sub_tasks. If
/// they do that then we end up with WorkGroup -> WorkItem. Either is just fine, we just have
/// to take it into account when 'parsing' the work.
#[instrument]
pub fn templates_to_work(templates: Vec<Template>) -> Result<Vec<sim_external::Work>, Error> {
    let events = templates.into_iter().map(template_to_event).collect()?;

    let mut result = Vec::with_capacity(templates.len());
    let current_event = events.next::<Option<TemplateEvent>>();
    loop {
        let next = events.next();
    
        match 

    }
}
