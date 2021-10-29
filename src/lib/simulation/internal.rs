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
use crate::lib::simulation::external;
use derive_more::Display;
use std::collections::HashMap;

#[derive(Display, Debug)]
#[display(
    fmt = "Indexes {{simulation: {}, work_items_for_group: {:?}, work_by_id: {:?}}}",
    simulation,
    work_items_for_group,
    work_items_by_id
)]
pub struct Indexes<'a> {
    pub simulation: &'a external::Simulation,
    pub work_items_for_group: HashMap<&'a external::WorkGroupId, Vec<&'a external::WorkItem>>,
    pub work_items_by_id: HashMap<&'a external::WorkItemId, &'a external::WorkItem>,
}
