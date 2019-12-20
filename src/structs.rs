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

    /// Get an iterator of non-mutable references to `Task` items in the `TaskListing`
    pub fn task_iter(&self) -> std::slice::Iter<Task> {
        self.all_tasks.iter()
    }

    /// Get an iterator of mutable references to `Task` items in the `TaskListing`
    pub fn task_iter_mut(&mut self) -> std::slice::IterMut<Task> {
        self.all_tasks.iter_mut()
    }

    /// Move a task from one index to another. This will cause the element that came after `to` to
    /// get shifted towards the end (likewise for all subsequent elements)
    // TODO: have this return a Result so that the caller doesn't have to do bounds-checking
    pub fn move_task(&mut self, from: usize, to: usize) {
        let element_moving = self.all_tasks.remove(from);
        self.all_tasks.insert(to, element_moving);
    }

    /// List all tasks for today (with completion status, times, and note on which task is next)
    pub fn list_for_today(&self) {
        // Calculate some field widths
        let indent_size = 4;
        let description_width = ((self.task_iter().fold(0, |max, task| {
            let curr_len = task.details().unwrap().description().chars().count();
            if max > curr_len {
                max
            } else {
                curr_len
            }
        }) / indent_size)
            + 1)
            * indent_size;
        let id_width = ((self.task_iter().count().to_string().chars().count() / indent_size) + 1)
            * indent_size;
        let mut next_marked = false;

        // Display tasks
        for (n, task) in self.task_iter().enumerate() {
            // Check box
            if task.completed_today().is_some() {
                print!("{:<4}", "[x]");
            } else {
                print!("{:<4}", "[ ]")
            }

            // Numeric ID (used for "order" subcommand)
            print!("{:<width$}", n, width = id_width);

            // Description
            print!(
                "{:<width$}",
                task.details().unwrap().description(),
                width = description_width,
            );

            // Completion time
            let timestamp_display: String;
            if task.completed_today().is_some() {
                let datetime = task.completed_today().unwrap();
                timestamp_display = format!("{:02}:{:02}", datetime.hour(), datetime.minute());
            } else {
                timestamp_display = "--:--".into();
            }
            print!(
                "{:<width$}",
                timestamp_display,
                width = ((timestamp_display.chars().count() / indent_size) + 1) * indent_size
            );

            // Mark next task to be done
            if !next_marked && task.completed_today().is_none() {
                next_marked = true;
                print!("(next)");
            }

            println!();
        }
    }

    pub fn history_for_range(&self, start: Date<Local>, end: Date<Local>) {
        // Calculate some field widths
        let indent_size = 4;
        let description_width = ((self.task_iter().fold(0, |max, task| {
            let curr_len = task.details().unwrap().description().chars().count();
            if max > curr_len {
                max
            } else {
                curr_len
            }
        }) / indent_size)
            + 1)
            * indent_size;
        let id_width = ((self.task_iter().count().to_string().chars().count() / indent_size) + 1)
            * indent_size;

        // Print header row
        // TODO: break up rendering into multiple rows once this gets to the point that we care
        // about terminal width
        print!("{}{}", " ".repeat(id_width), " ".repeat(description_width));

        let mut date_at = start.clone();

        while date_at != end.succ() {
            print!(
                "{:<width$}",
                format!("{:<02}", date_at.day()),
                width = indent_size
            );

            date_at = date_at.succ();
        }
        println!();

        for (n, task) in self.task_iter().enumerate() {
            // Numeric ID
            print!("{:<width$}", n, width = id_width);
            // Description
            print!(
                "{:<width$}",
                task.details().unwrap().description(),
                width = description_width
            );

            // TODO: break the renderer out into a separate module, going to need a state machine
            // to get the kind of rendering desired
            date_at = start.clone();
            while date_at != end.succ() && date_at != Local::today().succ() {
                if task.completed_on(date_at) {
                    print!("o   ");
                } else {
                    if date_at == Local::today() {
                        print!("[ ] ");
                    } else if date_at > Local::today() {
                        print!("    ");
                    } else {
                        print!("x   ")
                    }
                }
                date_at = date_at.succ();
            }
            println!();
        }
    }
}

/// Represents a `Task` being completed on a particular day.
#[derive(Debug, Serialize, Deserialize)]
pub struct Completion {
    /// Date and time at which this completion was recorded
    datetime: DateTime<Utc>,

    /// User can make an optional remark when marking a task as complete
    // TODO: a list of these at some particular time should be supported
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

impl TaskDetails {
    /// Get a reference to the `description` string for this `Task`
    pub fn description(&self) -> &String {
        &self.description
    }
}

/// Errors for `Task` operations
#[derive(Debug)]
pub enum TaskError {
    /// User tried to complete a task that was already completed for today
    AlreadyCompleted,
    /// User tried to do something to a task that didn't exist
    NotFound,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TaskError::AlreadyCompleted => f.write_str("AlreadyCompleted"),
            TaskError::NotFound => f.write_str("NotFound"),
        }
    }
}

impl Error for TaskError {
    fn description(&self) -> &str {
        match self {
            TaskError::AlreadyCompleted => "Task was already completed",
            TaskError::NotFound => "Couldn't find task",
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
    pub fn details(&self) -> Option<&TaskDetails> {
        self.detail_history.first()
    }

    /// Returns true if completed on the given date
    pub fn completed_on(&self, date: Date<Local>) -> bool {
        for completion in &self.completions {
            let completion_date_utc: Date<Utc> = completion.datetime.date();
            let completion_date_local = completion_date_utc.with_timezone(&Local);

            if date == completion_date_local {
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

    /// Mark a task as complete for today
    pub fn mark_complete(&mut self) -> Result<(), TaskError> {
        if self.completed_today().is_some() {
            return Err(TaskError::AlreadyCompleted);
        }

        self.completions.push(Completion {
            datetime: Utc::now(),
            // TODO: support remarks
            remark: None,
        });

        return Ok(());
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
