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
use std::error::Error;
use std::fmt;

/// A remark on some task. It's used in two ways:
///
/// 1. associated with a `Completion` (this can only be done when completing the task)
/// 2. associated with the `Task` on some given day
#[derive(Debug, Serialize, Deserialize)]
pub struct Remark {
    /// Timestamp for when remark was made
    datetime: DateTime<Utc>,
    /// The remark itself
    remark: String,
}

/// Represents a `Task` being completed on a particular day.
#[derive(Debug, Serialize, Deserialize)]
pub struct Completion {
    /// Date and time at which this completion was recorded
    datetime: DateTime<Utc>,

    /// User can make an optional remark when marking a task as complete, later remarks are closer
    /// to the end of the list
    remark: Option<Remark>,
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

impl TaskDetails {
    /// Get a reference to the `description` string for this `Task`
    pub fn description(&self) -> &String {
        &self.description
    }
}

/// Errors for `Task` operations
// TODO: this mixes operations on both `Task` and `TaskListing`, and should probably be cleaned up.
#[derive(Debug)]
pub enum TaskError {
    /// User tried to complete a task that was already completed for today
    AlreadyCompleted,
    /// User tried to do something to a task that didn't exist
    NotFound,
    /// User tried to move a task to an index it's already at
    RedundantMove,
    /// Failed to store a TaskListing to disk
    StoreFailed,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TaskError::AlreadyCompleted => f.write_str("AlreadyCompleted"),
            TaskError::NotFound => f.write_str("NotFound"),
            TaskError::RedundantMove => f.write_str("RedundantMove"),
            TaskError::StoreFailed => f.write_str("StoreFailed"),
        }
    }
}

impl Error for TaskError {
    fn description(&self) -> &str {
        match self {
            TaskError::AlreadyCompleted => "Task was already completed",
            TaskError::NotFound => "Couldn't find task",
            TaskError::RedundantMove => "Can't move task to its own index",
            TaskError::StoreFailed => "Can't store task data to disk",
        }
    }
}

/// Represents a task. It includes a history of revisions to task details, as well as a list of
/// dates and times on which the task was completed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    /// A record of revisions made to the TaskDetails for this Task
    detail_history: Vec<TaskDetails>,

    /// A record of completions
    completions: Vec<Completion>,

    /// A record of remarks made on tasks
    #[serde(default = "Vec::new")]
    remarks: Vec<Remark>,
}

impl Default for Task {
    fn default() -> Self {
        Task {
            detail_history: Vec::new(),
            completions: Vec::new(),
            remarks: Vec::new(),
        }
    }
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
            remarks: Vec::new(),
        }
    }

    /// Get the current details for this Task
    pub fn details(&self) -> Option<&TaskDetails> {
        self.detail_history.first()
    }

    pub fn description(&self) -> &String {
        self.details().unwrap().description()
    }

    /// Returns true if task existed on the given date
    pub fn existed_on(&self, date: Date<Local>) -> bool {
        let dt_cmp: DateTime<Local> = Local
            .ymd(date.year(), date.month(), date.day())
            .and_hms(0, 0, 0);

        let dt_cmp_utc: DateTime<Utc> = dt_cmp.with_timezone(&Utc);
        let dt_created_utc: DateTime<Utc> = self.created().unwrap();

        let date_cmp_utc: Date<Utc> = dt_cmp_utc.date();
        let date_created_utc: Date<Utc> = dt_created_utc.date();

        if date_cmp_utc < date_created_utc {
            return false;
        }

        true
    }

    /// Returns true if completed on the given date
    pub fn completed_on(&self, date: Date<Local>) -> bool {
        for completion in &self.completions {
            let completion_date_utc: DateTime<Utc> = completion.datetime;
            let completion_date_local: DateTime<Local> = completion_date_utc.with_timezone(&Local);

            if date == completion_date_local.date() {
                return true;
            }
        }

        false
    }

    /// Optionally returns a `DateTime<Local>` for when this task was completed today (if it was),
    /// otherwise `None`
    // TODO: this should be `completed_today_at`, and another function `completed_today` should
    // return bool
    pub fn completed_today(&self) -> Option<DateTime<Local>> {
        let today: Date<Local> = Local::today();
        for completion in &self.completions {
            // Note to self: if you want to do timezone conversion with chrono, you have to convert
            // as a DateTime first, then get the dates with .date()
            let completion_dt_utc: DateTime<Utc> = completion.datetime;
            let completion_dt_local: DateTime<Local> = completion_dt_utc.with_timezone(&Local);

            if today == completion_dt_local.date() {
                return Some(completion_dt_local);
            }
        }

        None
    }

    /// Add a remark to a completed task (note: this isn't associated with a `Completion`)
    pub fn add_remark(&mut self, remark: String) -> Result<(), TaskError> {
        self.remarks.push(Remark {
            datetime: Utc::now(),
            remark,
        });

        Ok(())
    }

    /// Mark a task as complete for today
    pub fn mark_complete(&mut self, remark: &Option<String>) -> Result<(), TaskError> {
        if self.completed_today().is_some() {
            return Err(TaskError::AlreadyCompleted);
        }

        let now = Utc::now();

        let remark: Option<Remark> = if let Some(remark) = remark {
            Some(Remark {
                datetime: now,
                remark: remark.to_string(),
            })
        } else {
            None
        };

        self.completions.push(Completion {
            datetime: now,
            remark: remark,
        });

        return Ok(());
    }

    /// Get the timestamp at which the Task was first created
    fn created(&self) -> Option<DateTime<Utc>> {
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
