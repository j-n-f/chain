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
use dirs;
use ron;
use ron::de::Error as RonError;
use std::error::Error;
use std::fs::create_dir;
use std::fs::OpenOptions;
use std::io::prelude::*;
use structopt::StructOpt;

mod structs;
mod tui;

use structs::{TaskListing, TaskOperation};

/// This allows parsing date strings into `Opt`
#[derive(Debug)]
struct LocalDate {
    date: Date<Local>,
}

impl std::str::FromStr for LocalDate {
    type Err = chrono::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // chrono is complicated
        let dt: NaiveDate = NaiveDate::parse_from_str(s, "%F").expect("couldn't parse date");
        let local: Date<Utc> = Date::<Utc>::from_utc(dt, Utc);
        Ok(LocalDate {
            date: local.with_timezone(&Local),
        })
    }
}

/// Configuration for `structopt`
#[derive(StructOpt, Debug)]
#[structopt(name = "chain", about = "daily task tracking")]
enum Opt {
    #[structopt(name = "new", about = "create a new task")]
    New { description: String },
    #[structopt(name = "today", about = "view task status for today")]
    Today,
    #[structopt(name = "move", about = "move a task from some position to another")]
    Move { from: usize, to: usize },
    #[structopt(name = "done", about = "mark a task as complete for today")]
    Done { index: usize },
    #[structopt(name = "history", about = "show history of task completion")]
    History { start: LocalDate, end: LocalDate },
    #[structopt(name = "tui", about = "launch text ui")]
    Tui,
}

/// Ensures that the folder for `TASK_FILE` exists, creates it if it doesn't, and similarly loads
/// up any existing task data, returning it as a `TaskListing` for the caller. If `TASK_FILE`
/// doesn't yet exist, it initializes it as an empty file.
fn init_task_listing() -> TaskListing {
    // Construct a path to the data file used to persist tasks between invocations
    let tasks_path = structs::tasklisting::get_tasks_path();

    // TODO: note that the file doesn't initially exist (if so), so that later error handling can
    // know if errors are expected

    // Create task file if it doesn't exist, then open it (note, need write(true) for file
    // creation)
    let mut tasks_file = match OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .open(&tasks_path)
    {
        Err(e) => panic!(
            "couldn't open {}: {}; {:?}",
            tasks_path.to_str().unwrap(),
            e.description(),
            e
        ),
        Ok(file) => file,
    };

    // Load existing tasks data
    let mut tasks_file_string = String::new();
    match tasks_file.read_to_string(&mut tasks_file_string) {
        Err(e) => panic!(
            "couldn't read {}: {}",
            tasks_path.to_str().unwrap(),
            e.description()
        ),
        Ok(_) => (),
    }

    // TODO: explicitly check that a file was just created before silently handling errors
    let tasks: TaskListing = match ron::de::from_str(&tasks_file_string) {
        Err(e) => match e {
            RonError::IoError(s) => panic!("RON deserialization IoError: {}", s),
            RonError::Message(s) => panic!("RON deserialization Message: {}", s),
            RonError::Parser(e, pos) => match e {
                ron::de::ParseError::ExpectedUnit => {
                    if pos.col == 1 && pos.line == 1 {
                        // Empty file was just created, we can ignore this
                        TaskListing::new()
                    } else {
                        panic!("RON Parser error at line {}, col {}", pos.line, pos.col);
                    }
                }
                ron::de::ParseError::ExpectedStruct => {
                    // No struct was found, file was just created
                    TaskListing::new()
                }
                _ => panic!("Unhandled RON parser error: {:?}", e),
            },
        },
        Ok(tasks) => tasks,
    };

    tasks
}

fn main() {
    // If the data folder doesn't exist, create it
    let mut data_path = dirs::data_dir().unwrap();
    data_path.push("chain");

    if !data_path.exists() {
        println!("{:?} doesn't exist, creating", data_path);
        match create_dir(&data_path) {
            Err(e) => panic!("couldn't create {:?}: {:?}", &data_path, e),
            Ok(_) => println!("created {:?}", &data_path),
        }
    }

    // Initialize the `TaskListing` before parsing command args
    let mut tasks: TaskListing = init_task_listing();

    // We may run a command that indicates a single operation to perform
    let mut operation: Option<TaskOperation> = None;

    // We may want to show a user the updated task listing after operation is complete
    let mut list_after = false;

    // Handle manipulation of `TaskListing` according to command line args given
    match Opt::from_args() {
        // Create a new task
        Opt::New { description } => {
            println!("new task: {}", description);

            operation = Some(TaskOperation::Add { description });

            list_after = true;
        }
        // Display tasks that need to be done today
        Opt::Today => {
            // Display header
            println!();
            println!("Task status for {}", Local::today().format("%F"));
            println!();

            list_after = true;
        }
        // Re-order tasks
        Opt::Move { from, to } => {
            operation = Some(TaskOperation::Reorder { from, to });

            list_after = true;
        }
        // Mark a task as done for the day
        Opt::Done { index } => {
            operation = Some(TaskOperation::MarkComplete {
                task_index: index,
                remark: None,
            });

            list_after = true;
        }
        Opt::History { start, end } => {
            // TODO: this one is an oddball, perhaps each arm should return an enumerated value
            // describing the report to be shown afterward a command is processed
            let start = start.date;
            let end = end.date;

            let mut error = false;

            if start > end {
                error = true;
                println!("error: start comes after end");
            }

            if !error {
                let num_days = end.signed_duration_since(start).num_days() + 1;
                let s_if_plural = if num_days > 1 { "s" } else { "" };
                let today_if_end_is_today = if end == Local::today() { "(today)" } else { "" };

                println!();
                println!(
                    "{} day{} of History from {} to {} {}",
                    num_days,
                    s_if_plural,
                    start.format("%F"),
                    end.format("%F"),
                    /* need to lop off timezone */ today_if_end_is_today
                );
                println!();

                tasks.history_for_range(start, end);
            }
        }
        Opt::Tui => {
            // TODO: have this arm run when no argument is provided (i.e. `chain tui` and `chain`
            // are equivalent)

            // NOTE: this will run its own loop, and create a stream of TaskOperation which will be
            // handled by TaskListing internally
            tui::run(&mut tasks);
        }
    };

    // Handle an operation if the command wasn't merely to display information
    let mut modifications_made: bool = false;
    if let Some(op) = operation {
        match tasks.handle_operation(op) {
            Err(e) => {
                println!("error: {}", e.description());
            }
            Ok(_) => modifications_made = true,
        }
    }

    if list_after {
        match Opt::from_args() {
            Opt::Today => {
                // Always causes listing to be displayed
                tasks.list_for_today();
            }
            Opt::Done { .. } | Opt::Move { .. } | Opt::New { .. } if modifications_made => {
                // Only display the listing if something changed
                tasks.list_for_today();
            }
            _ => (),
        }
    }

    match tasks.store(structs::tasklisting::get_tasks_path()) {
        Err(e) => println!("\nfailed to store tasks: {}", e.description()),
        Ok(_) if modifications_made => println!("\ntask database successfully updated"),
        Ok(_) => (),
    }

    // All done!
}
