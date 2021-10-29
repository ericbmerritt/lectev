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

///! This module provides a set of useful indexes on work and work groups that makes working with
///! the data much simpler.
use crate::lib::simulation::external::{
    Simulation, Work, WorkGroup, WorkGroupId, WorkItem, WorkItemId,
};
use derive_more::Display;
use std::collections::HashMap;
use tracing::instrument;

#[derive(Display, Debug)]
#[display(
    fmt = "Indices {{simulation: {}, work_items_for_group: {:?}, work_by_id: {:?}}}",
    simulation,
    work_items_for_group,
    work_items_by_id
)]
pub struct Indices<'a> {
    pub simulation: &'a Simulation,
    pub work_items_for_group: HashMap<&'a WorkGroupId, Vec<&'a WorkItem>>,
    pub work_items_by_id: HashMap<&'a WorkItemId, &'a WorkItem>,
}

#[instrument]
fn find_work_items_for_a_group<'a>(
    work_group: &'a WorkGroup,
) -> HashMap<&'a WorkGroupId, Vec<&'a WorkItem>> {
    let mut index = HashMap::new();

    for child in &work_group.children {
        match child {
            Work::WorkItem(item) => {
                index
                    .entry(&work_group.id)
                    .and_modify(|leaves: &mut Vec<&'a WorkItem>| leaves.push(item))
                    .or_insert_with(|| vec![item]);
            }
            Work::WorkGroup(group) => {
                let leaves = find_work_items_for_a_group(group);
                let mut these_leaves = Vec::with_capacity(leaves.len());

                for items in leaves.values() {
                    these_leaves.extend(items);
                }

                match index.get_mut(&work_group.id) {
                    Some(existing_leaves) => {
                        existing_leaves.extend(these_leaves);
                    }
                    None => {
                        index.insert(&work_group.id, these_leaves);
                    }
                }
                index.extend(leaves);
            }
        }
    }

    index
}

#[instrument]
fn build_items_for_group_index<'a>(
    sim: &'a Simulation,
) -> HashMap<&'a WorkGroupId, Vec<&'a WorkItem>> {
    let mut map = HashMap::new();

    for work in &sim.work {
        match work {
            Work::WorkItem(_) => continue,
            Work::WorkGroup(group) => map.extend(find_work_items_for_a_group(group)),
        }
    }

    map
}

#[instrument]
fn build_items_by_id_index_prime<'a>(
    group: &'a WorkGroup,
    acc: &mut HashMap<&'a WorkItemId, &'a WorkItem>,
) {
    for work in &group.children {
        match work {
            Work::WorkItem(item) => {
                acc.insert(&item.id, item);
            }
            Work::WorkGroup(group) => {
                build_items_by_id_index_prime(group, acc);
            }
        }
    }
}

#[instrument]
fn build_items_by_id_index<'a>(sim: &'a Simulation) -> HashMap<&'a WorkItemId, &'a WorkItem> {
    let mut map = HashMap::new();

    for work in &sim.work {
        match work {
            Work::WorkItem(ref item) => {
                map.insert(&item.id, item);
            }
            Work::WorkGroup(ref group) => {
                build_items_by_id_index_prime(group, &mut map);
            }
        }
    }

    map
}

#[instrument]
pub fn sim_to_indexes<'a>(sim: &'a Simulation) -> Indices<'a> {
    Indices {
        simulation: sim,
        work_items_for_group: build_items_for_group_index(sim),
        work_items_by_id: build_items_by_id_index(sim),
    }
}
