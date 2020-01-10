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
use ron::ser::{PrettyConfig, Serializer};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::PathBuf;

use super::Task;
use super::TaskError;
use super::TaskOperation;

/// name of file in which task data is stored
const TASK_FILE: &'static str = "taskdata.ron";

pub fn get_tasks_path() -> PathBuf {
    let mut tasks_path = dirs::data_dir().unwrap();
    tasks_path.push("chain");
    tasks_path.push(TASK_FILE);

    tasks_path
}

/// This struct exists so that the RON output used to store tasks between invocations can be
/// prefixed with the type name when serialized. (it was previously just a vector, but this made it
/// impossible to output human-readable RON).
///
/// It also represents the user's prioritization of tasks (based on the order they appear in the
/// vector)
#[derive(Serialize, Deserialize, Clone)]
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

    /// Handle an operation and store the result to disk
    pub fn handle_and_store(&mut self, op: &TaskOperation) -> Result<(), TaskError> {
        self.handle_operation(op)?;
        self.store(get_tasks_path())?;

        // TODO: reload from disk, as another command from CLI may have modified TaskListing
        // TODO: maybe there should be some kind of locking mechanism to avoid race conditions

        Ok(())
    }

    /// Handle an operation on the TaskListing. This will only update the listing in memory, it's
    /// the caller's responsibility to ensure it gets updated in persistent storage.
    pub fn handle_operation(&mut self, op: &TaskOperation) -> Result<(), TaskError> {
        match op {
            TaskOperation::Add { description } if description.chars().count() == 0 => {
                return Err(TaskError::MissingDescription);
            }
            TaskOperation::Add { description } => {
                let new_task = Task::new(description.to_string());
                self.push(new_task);
            }
            TaskOperation::MarkComplete { task_index, remark } => {
                // TODO: refactor everything up to "let matching_task" as self.task_from_index()?
                if *task_index >= self.all_tasks.iter().count() {
                    return Err(TaskError::NotFound);
                }

                let matching_task: &mut Task = self.task_iter_mut().nth(*task_index).unwrap();

                matching_task.mark_complete(remark)?
            }
            TaskOperation::Reorder { from, to } => self.move_task(*from, *to)?,
            TaskOperation::AddRemark { task_index, remark } => {
                // TODO: refactor everything up to "let matching_task" as self.task_from_index()?
                if *task_index >= self.all_tasks.iter().count() {
                    return Err(TaskError::NotFound);
                }

                let matching_task: &mut Task = self.task_iter_mut().nth(*task_index).unwrap();

                matching_task.add_remark(remark.to_string())?
            }
        }

        Ok(())
    }

    /// Serialize listing and write to disk
    pub fn store(&self, path: std::path::PathBuf) -> Result<(), TaskError> {
        let ron_config = PrettyConfig {
            ..Default::default()
        };
        let mut serializer = Serializer::new(Some(ron_config), true);

        // Run the serializer on our task data, get back a string
        // TODO: maybe the file should have a checksum so that we can detect corruption from manual
        // editing
        match self.serialize(&mut serializer) {
            Err(e) => match e {
                ron::ser::Error::Message(s) => panic!("RON serialization error: {}", s),
            },
            Ok(_) => {}
        }
        let serialized = serializer.into_output_string();

        // Write the serialized data to chain's data folder
        let task_file_open = OpenOptions::new()
            .write(true)
            .truncate(true) // truncate, or else the file will be appended to
            .open(&path);

        match task_file_open {
            Err(_e) => {
                return Err(TaskError::StoreFailed);
            }
            Ok(mut file) => match file.write_all(serialized.as_bytes()) {
                Ok(_) => return Ok(()),
                Err(_e) => return Err(TaskError::StoreFailed),
            },
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

    /// Get a reference to a task by index
    #[allow(dead_code)]
    pub fn task_from_index(&mut self, index: usize) -> Option<&mut Task> {
        if index >= self.total_tasks() {
            return None;
        }

        self.all_tasks.iter_mut().nth(index)
    }

    /// Move a task from one index to another. This will cause the element that came after `to` to
    /// get shifted towards the end (likewise for all subsequent elements)
    pub fn move_task(&mut self, from: usize, to: usize) -> Result<(), TaskError> {
        if from >= self.total_tasks() || to >= self.total_tasks() {
            return Err(TaskError::NotFound);
        }

        if from == to {
            return Err(TaskError::RedundantMove);
        }

        let element_moving = self.all_tasks.remove(from);
        self.all_tasks.insert(to, element_moving);

        Ok(())
    }

    /// Get the total number of tasks in the listing
    pub fn total_tasks(&self) -> usize {
        self.task_iter().count()
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

        let mut dates: Vec<Date<Local>> = Vec::new();
        let mut date_at = start.clone();

        while date_at != end.succ() {
            dates.push(date_at);
            date_at = date_at.succ();
        }

        for date in dates.iter() {
            print!(
                "{:<width$}",
                format!("|{:<02}", date.day()),
                width = indent_size + 1
            );
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
            let mut any_done = false;
            let mut last_complete = false;
            for date in dates.iter() {
                print!("|");

                if date <= &Local::today() {
                    if task.completed_on(*date) {
                        print!("o");
                        last_complete = true;
                        any_done = true;
                    } else if (date != &Local::today()) && !any_done {
                        print!(" ");
                    } else if date == &Local::today() {
                        print!("?");
                    } else if any_done && last_complete {
                        print!("x");
                        last_complete = false;
                    }

                    if date != dates.last().unwrap() {
                        if last_complete && (date != &Local::today()) {
                            print!("-o-");
                        } else {
                            print!("   ");
                        }
                    }
                } else {
                    print!("    ");
                }
            }
            println!();
        }
    }
}
