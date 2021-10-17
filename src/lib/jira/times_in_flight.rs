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
use crate::lib::jira::core;
use bdays::HolidayCalendar;
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::instrument;
use uom::si::f64::Time;
use uom::si::time::day;
use url::Url;

#[derive(Debug, Serialize)]
struct WorkingEntry<'a> {
    item: &'a core::Item,
    todo: Time,
    ready: Time,
    in_dev: Time,
    in_test: Time,
    waiting: Time,
    completed: Time,
    oldest_estimate: Option<Time>,
}

#[derive(Debug, Serialize)]
pub struct Entry<'a> {
    pub url: String,
    pub name: &'a str,
    pub description: &'a str,
    pub todo: f64,
    pub ready: f64,
    pub in_dev: f64,
    pub in_test: f64,
    pub waiting: f64,
    pub completed: f64,
    pub first_estimate: Option<f64>,
    pub status: &'a core::ItemStatus,
    pub resolution: &'a core::Resolution,
}

#[instrument]
fn get_business_days(start: &DateTime<Utc>, end: &DateTime<Utc>) -> Time {
    let cal = bdays::calendars::us::USSettlement;
    Time::new::<day>(f64::from(cal.bdays(*start, *end)))
}

#[instrument]
fn set_days(entry: &mut WorkingEntry, status: &core::ItemStatus, days: Time) {
    match status {
        core::ItemStatus::ToDo => entry.todo += days,
        core::ItemStatus::Ready => entry.ready += days,
        core::ItemStatus::InDev => entry.in_dev += days,
        core::ItemStatus::InTest => entry.in_test += days,
        core::ItemStatus::Waiting => entry.waiting += days,
        core::ItemStatus::Completed => entry.completed += days,
    }
}

#[instrument]
fn get_latest_estimate(
    old: Option<core::ItemTimeLineEntry>,
    new: &core::ItemTimeLineEntry,
) -> Option<core::ItemTimeLineEntry> {
    match (&old, new) {
        (
            Some(core::ItemTimeLineEntry::Estimate {
                start: old_start, ..
            }),
            core::ItemTimeLineEntry::Estimate {
                start: new_start, ..
            },
        ) if old_start > new_start => Some(new.clone()),
        (
            Some(core::ItemTimeLineEntry::Estimate {
                start: old_start, ..
            }),
            core::ItemTimeLineEntry::Estimate {
                start: new_start, ..
            },
        ) if old_start < new_start => old,
        (None, _) => Some(new.clone()),
        _ => old,
    }
}

#[instrument]
fn calculate_time_in_flight<'a>(item: &'a core::Item) -> WorkingEntry<'a> {
    let mut entry = WorkingEntry {
        item,
        todo: Time::new::<day>(0.0),
        ready: Time::new::<day>(0.0),
        in_dev: Time::new::<day>(0.0),
        in_test: Time::new::<day>(0.0),
        waiting: Time::new::<day>(0.0),
        completed: Time::new::<day>(0.0),
        oldest_estimate: None,
    };

    let now = Utc::now();
    let mut oldest_estimate = None;

    for timeline_entry in &item.timeline {
        match timeline_entry {
            core::ItemTimeLineEntry::OpenStatus { status, start } => {
                set_days(&mut entry, status, get_business_days(start, &now));
            }

            core::ItemTimeLineEntry::ClosedStatus { status, start, end } => {
                set_days(&mut entry, status, get_business_days(start, end));
            }

            new_estimate @ core::ItemTimeLineEntry::Estimate { .. } => {
                oldest_estimate = get_latest_estimate(oldest_estimate, new_estimate);
            }
        }
    }
    entry.oldest_estimate = oldest_estimate.and_then(|estimate| {
        if let core::ItemTimeLineEntry::Estimate { days, .. } = estimate {
            Some(days)
        } else {
            None
        }
    });

    entry
}

#[instrument]
fn prepare_for_display<'a>(base_url: &Url, entry: WorkingEntry<'a>) -> Entry<'a> {
    let url = format!("{}browse/{}", base_url.as_str(), &entry.item.name);

    Entry {
        url,
        name: &entry.item.name,
        description: &entry.item.description,
        todo: entry.todo.get::<day>(),
        ready: entry.ready.get::<day>(),
        in_dev: entry.in_dev.get::<day>(),
        in_test: entry.in_test.get::<day>(),
        waiting: entry.waiting.get::<day>(),
        completed: entry.completed.get::<day>(),
        first_estimate: entry.oldest_estimate.map(|estimate| estimate.get::<day>()),
        status: &entry.item.status,
        resolution: &entry.item.resolution,
    }
}

#[instrument]
pub fn calculate<'a>(instance_url: &Url, items: &'a [core::Item]) -> Vec<Entry<'a>> {
    items
        .iter()
        .map(calculate_time_in_flight)
        .map(|working_entry| prepare_for_display(instance_url, working_entry))
        .collect()
}
