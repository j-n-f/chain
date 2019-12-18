/*
 * Copyright (c) 2019 John Ferguson
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

// TODO: break these out into individual files

/// This struct exists so that the RON output used to store tasks between invocations can be
/// prefixed with the type name when serialized. (it was previously just a vector, but this made it
/// impossible to output human-readable RON).
///
/// It also represents the user's prioritization of tasks (based on the order they appear in the
/// vector.
#[derive(Serialize, Deserialize)]
pub struct TaskListing {
    all_tasks: Vec<Task>,
}

impl TaskListing {
    /// Create a new `TaskListing`
    pub fn new() -> TaskListing {
        TaskListing {
            all_tasks: Vec::new(),
        }
    }

    /// Push a new `Task` into the `TaskListing`
    pub fn push(&mut self, task: Task) {
        self.all_tasks.push(task);
    }
}

/// Represents a `Task` being completed on a particular day.
#[derive(Debug, Serialize, Deserialize)]
pub struct Completion {
    /// Date and time at which this completion was recorded
    datetime: DateTime<Utc>,

    /// User can make an optional remark when marking a task as complete
    remark: Option<String>,
}

/// Represents the state of a task at some point in time (i.e. the user can change the
/// description).
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskDetails {
    /// Timestamp of when these details described the Task
    revised: DateTime<Utc>,

    /// A monotonically increasing revision ID
    revision_id: u64,

    /// A description of the task/condition
    description: String,

    /// None => time of day doesn't matter, else: this task needs to be completed by a particular
    /// time of day
    // TODO: find the best struct/library to represent this kind of value
    sync_time: Option<u32>, /* time of day */
}

/// Represents a task. It includes a history of revisions to task details, as well as a list of
/// dates and times on which the task was completed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    /// A record of revisions made to the TaskDetails for this Task
    detail_history: Vec<TaskDetails>,

    /// A record of completions
    completions: Vec<Completion>,
}

impl Task {
    /// Create a new Task
    pub fn new(description: String) -> Task {
        let details = TaskDetails::new(None, 0, description);
        let mut detail_history = Vec::new();
        detail_history.push(details);

        Task {
            detail_history,
            completions: Vec::new(),
        }
    }

    /// Get the current details for this Task
    // TODO: remove this after using
    #[allow(dead_code)]
    fn details(&self) -> Option<&TaskDetails> {
        self.detail_history.first()
    }

    /// Get the timestamp at which the Task was first created
    // TODO: remove this after using
    #[allow(dead_code)]
    fn created(self) -> Option<DateTime<Utc>> {
        // Look up the oldest revision for this task, and return its `revised` timestamp
        match self.detail_history.last() {
            Some(details) => Some(details.revised),
            None => None,
        }
    }
}

impl TaskDetails {
    fn new(time: Option<DateTime<Utc>>, revision_id: u64, description: String) -> TaskDetails {
        TaskDetails {
            revised: time.unwrap_or(Utc::now()),
            revision_id,
            description,
            sync_time: None,
        }
    }
}
