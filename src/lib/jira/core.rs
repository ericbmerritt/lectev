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
use chrono::prelude::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use uom::si::f64::Time;
use url::Url;
use uuid::Uuid;

/// Id of the item
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ItemId(pub Uuid);

#[derive(Display, Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NativeId(pub String);

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct ItemTimeLineEntryId(pub Uuid);

/// Provides the potential resolutions for an issue
#[derive(Display, Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    UnResolved,
    Rejected,
    Delivered,
}

/// Provides the internal representation of status' for an item
#[derive(Display, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemStatus {
    ToDo,
    Ready,
    InDev,
    InTest,
    Waiting,
    Completed,
}

/// Timeline entry
///
/// This currently only contains status' in the future it may contain other things.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ItemTimeLineEntry {
    /// ClosedStatus is for a status that is complete. Ie, the item has transitioned to a new status
    /// and this status will no longer be updated
    ClosedStatus {
        status: ItemStatus,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    /// An open status is a status that is not complete. Essentially, the item is still in this
    /// status at the time the report was run
    OpenStatus {
        status: ItemStatus,
        start: DateTime<Utc>,
    },
    Estimate {
        start: DateTime<Utc>,
        days: Time,
    },
}
#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ItemType {
    Operational,
    Reinvestment,
    Feature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub native_id: NativeId,
    pub native_url: Url,
    pub name: String,
    pub description: String,
    pub typ: ItemType,
    pub status: ItemStatus,
    pub resolution: Resolution,
    pub timeline: Vec<ItemTimeLineEntry>,
}
