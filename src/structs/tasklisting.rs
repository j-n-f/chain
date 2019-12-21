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

use super::task::Task;

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

        let mut dates: Vec<Date<Local>> = Vec::new();
        let mut date_at = start.clone();

        while date_at != end.succ() {
            dates.push(date_at);
            date_at = date_at.succ();
        }

        for date in dates.iter() {
            print!(
                "{:<width$}",
                format!("{:<02}", date.day()),
                width = indent_size
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
            for date in dates.iter() {
                if task.completed_on(*date) {
                    print!("o   ");
                } else {
                    if date == &Local::today() {
                        print!("[ ] ");
                    } else if date > &Local::today() {
                        print!("    ");
                    } else {
                        print!("x   ")
                    }
                }
            }
            println!();
        }
    }
}
