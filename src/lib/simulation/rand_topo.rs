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

//! # Randomized Topological Sort
//!
//! This module provides a randomized topological sort based on
//! [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting#Kahn.27s_algorithm).
//! The goal is to provide a randomized sorting where the dependencies are still respected.
//! This module provides the data types associated with a simulation
use crate::lib::simulation::external::{Work, WorkGroup, WorkItemId, WorkItemOrGroupId};
use crate::lib::simulation::index;
use rand::seq::SliceRandom;
use retain_mut::RetainMut;
use snafu::{OptionExt, Snafu};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

/// Enumerates the errors provided by this module
#[derive(Debug, Snafu)]
pub enum Error {
    /// Returned if there is a cycle in the dependencies.
    #[snafu(display("Cycle detected in the dependencies"))]
    CycleDetected,
    /// Returned if we unexpectedly run out of sorted heads on the sorted stack. There shouldn't be
    /// any way for this to happen in the normal course of events.
    #[snafu(display("Unexpected empty stack of elements"))]
    EmptyStack,
}

/// This holds a flattened work item. The dependencies are the dependencies of
/// the work item itself and its parents.
#[derive(Debug, Clone)]
struct WorkItemFlatDeps<'a> {
    work_item_id: &'a WorkItemId,
    dependencies: HashSet<&'a WorkItemId>,
}

/// This holds the inverse of [`WorkItemFlatDeps`]. It contains a work item with all of its
/// incoming dependencies rather than outgoing dependencies. Humans tend to think in terms of
/// outgoing dependencies, so the external simulation takes them in that way. The sorting algorithm
/// we use needs incoming dependencies.
#[derive(Debug, Clone)]
struct WorkItemIncomingLinks<'a> {
    work_item_id: &'a WorkItemId,
    incoming_links: HashSet<&'a WorkItemId>,
}

/// Represents a prepared list of work items that can be sorted. The caller can reuse this set of
/// work items to sort as many times as they wish.
#[derive(Debug, Clone)]
pub struct Prepared<'a> {
    elements: Vec<WorkItemIncomingLinks<'a>>,
}

/// Given an id that may be a [`WorkGroupId`] or a [`WorkItemId`] return all the [`WorkItemId`]s
/// associated with them. For a [`WorkGroupId`] that vector will include all of the [`WorkItemId`]s
/// associated with both children and descendants. The majority of the work to map [`WorkGroupId`]
/// to the [`WorkItemId`] is defined in the [`index`] module.
#[instrument]
fn resolve_dependency_ids<'a>(
    indexes: &index::Indices<'a>,
    id: &'a WorkItemOrGroupId,
) -> Vec<&'a WorkItemId> {
    match id {
        WorkItemOrGroupId::WorkGroup(ref group_id) => indexes
            .work_items_for_group
            .get(group_id)
            .map_or(Vec::with_capacity(0), |data| {
                data.iter().map(|item| &item.id).collect()
            }),
        WorkItemOrGroupId::WorkItem(ref item_id) => vec![item_id],
    }
}

/// Given a vector of [`WorkItemOrGroupId`], resolve all the related [`WorkItemId`]s.
fn resolve_all_dependencies<'a>(
    indexes: &index::Indices<'a>,
    ids: &'a [WorkItemOrGroupId],
) -> HashSet<&'a WorkItemId> {
    ids.iter()
        .flat_map(|id| resolve_dependency_ids(indexes, id))
        .collect()
}
/// This function flattens the dependency tree, pushing dependencies at the node level to
/// dependencies at the leaves.
/// This is a recursive function, but the depth of the tree should never reach a point where the
/// stack size becomes a problem.
#[instrument]
fn flatten_group<'a>(
    indexes: &index::Indices<'a>,
    work_group: &'a WorkGroup,
    deps: HashSet<&'a WorkItemId>,
) -> Vec<WorkItemFlatDeps<'a>> {
    let group_deps = resolve_all_dependencies(indexes, &work_group.dependencies);

    let mut result = Vec::new();

    for work in &work_group.children {
        match work {
            Work::WorkGroup(ref group) => {
                result.append(&mut flatten_group(indexes, group, group_deps.clone()));
            }
            Work::WorkItem(ref item) => {
                let deps = group_deps
                    .union(&resolve_all_dependencies(indexes, &item.dependencies))
                    .copied()
                    .collect();
                result.push(WorkItemFlatDeps {
                    work_item_id: &item.id,
                    dependencies: deps,
                });
            }
        }
    }

    result
}

/// This function converts the data pointed to by an index to a vector of [`WorkItemFlatDeps`].
/// Essentially, it flattens out the groups and ends up with just workable work.
fn index_to_flat_deps<'a>(indices: &index::Indices<'a>) -> Vec<WorkItemFlatDeps<'a>> {
    let mut result = Vec::new();

    for work in &indices.simulation.work {
        match work {
            Work::WorkGroup(ref group) => {
                result.append(&mut flatten_group(indices, group, HashSet::new()));
            }
            Work::WorkItem(ref item) => {
                let deps = resolve_all_dependencies(indices, &item.dependencies);
                result.push(WorkItemFlatDeps {
                    work_item_id: &item.id,
                    dependencies: deps,
                });
            }
        }
    }

    result
}

/// This function inverts an array of [`WorkItemFlatDeps`] to an array of
/// [`WorkItemIncomingLinks`]. This sets up for the sorting process that needs incoming links
/// rather than out going deps.
fn outgoing_deps_to_incoming_deps<'a>(
    outgoing: &[WorkItemFlatDeps<'a>],
) -> Vec<WorkItemIncomingLinks<'a>> {
    let mut incoming = HashMap::with_capacity(outgoing.len());

    for item in outgoing {
        for dep in &item.dependencies {
            incoming
                .entry(dep)
                .and_modify(|leaves: &mut HashSet<&'a WorkItemId>| {
                    leaves.insert(item.work_item_id);
                })
                .or_insert({
                    let mut set = HashSet::with_capacity(1);
                    set.insert(item.work_item_id);
                    set
                });
        }
    }

    let mut result = Vec::with_capacity(incoming.len());
    for (work_item_id, incoming_links) in incoming {
        result.push(WorkItemIncomingLinks {
            work_item_id,
            incoming_links,
        });
    }

    result
}

/// Prepares the indices for sorting. The caller can call prepare once and then resort the prepared
/// instances as often as they like
pub fn prepare<'a>(indices: &index::Indices<'a>) -> Prepared<'a> {
    let flattened_deps = index_to_flat_deps(indices);
    Prepared {
        elements: outgoing_deps_to_incoming_deps(&flattened_deps),
    }
}

/// This function kicks of the topo sort algorithm by finding the elements with no incoming links.
fn find_and_load_no_incoming<'a>(
    items: &mut Vec<WorkItemIncomingLinks<'a>>,
    no_deps: &mut Vec<&'a WorkItemId>,
) {
    items.retain(|link| {
        if link.incoming_links.is_empty() {
            no_deps.push(link.work_item_id);
            false
        } else {
            true
        }
    });
}

/// Topo sort the elements with incoming links such that things are correctly sorted but with an
/// element of randomness
pub fn sort(mut prepared: Prepared) -> Result<Vec<&WorkItemId>, Error> {
    let mut rng = rand::thread_rng();

    let mut sorted_elements = Vec::with_capacity(prepared.elements.len());
    let mut no_deps = Vec::new();
    find_and_load_no_incoming(&mut prepared.elements, &mut no_deps);

    while !no_deps.is_empty() {
        no_deps.shuffle(&mut rng);
        let head = no_deps.pop().context(EmptyStack {})?;
        sorted_elements.push(head);
        prepared
            .elements
            .retain_mut(|link: &mut WorkItemIncomingLinks| {
                link.incoming_links.remove(head);
                if link.incoming_links.is_empty() {
                    no_deps.push(link.work_item_id);
                    false
                } else {
                    true
                }
            });
    }

    if prepared.elements.is_empty() {
        Ok(sorted_elements)
    } else {
        CycleDetected.fail()
    }
}
