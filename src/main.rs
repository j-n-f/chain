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
use serde::Serialize;
use std::error::Error;
use std::fs::create_dir;
use std::fs::OpenOptions;
use std::io::prelude::*;
use structopt::StructOpt;

mod structs;

use structs::{Task, TaskError, TaskListing};

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
}

/// name of file in which task data is stored
const TASK_FILE: &'static str = "taskdata.ron";

/// Ensures that the folder for `TASK_FILE` exists, creates it if it doesn't, and similarly loads
/// up any existing task data, returning it as a `TaskListing` for the caller. If `TASK_FILE`
/// doesn't yet exist, it initializes it as an empty file.
fn init_task_listing() -> TaskListing {
    // Construct a path to the data file used to persist tasks between invocations
    let mut tasks_path = dirs::data_dir().unwrap();
    tasks_path.push("chain");
    tasks_path.push(TASK_FILE);

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
        Err(e) => panic!("couldn't open {}: {}; {:?}", TASK_FILE, e.description(), e),
        Ok(file) => file,
    };

    // Load existing tasks data
    let mut tasks_file_string = String::new();
    match tasks_file.read_to_string(&mut tasks_file_string) {
        Err(e) => panic!("couldn't read {}: {}", TASK_FILE, e.description()),
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

    // Handle manipulation of `TaskListing` according to command line args given
    match Opt::from_args() {
        // Create a new task
        Opt::New { description } => {
            println!("new task: {}", description);

            let new_task = Task::new(description);
            println!("{:?}", new_task);
            tasks.push(new_task);
        }
        // Display tasks that need to be done today
        Opt::Today => {
            // Display header
            println!();
            println!("Task status for {}", Local::today().format("%F"));
            println!();

            tasks.list_for_today();
        }
        // Re-order tasks
        Opt::Move { from, to } => {
            // check that the values are in range
            let num_tasks = tasks.task_iter().count();
            let max_index = num_tasks - 1;
            let mut error = false;

            if from > max_index || to > max_index {
                println!(
                    "error: index out of range, values should be between 0 and {}",
                    max_index
                );
                error = true;
            } else {
                if from == to {
                    println!("error: indexes are the same");
                    error = true;
                } else {
                    // We have valid indexes, perform the swap
                    println!();
                    println!(
                        "Bumping \"{}\" to position {}",
                        tasks
                            .task_iter()
                            .nth(from)
                            .unwrap()
                            .details()
                            .unwrap()
                            .description(),
                        to
                    );
                    println!();

                    tasks.move_task(from, to);
                }
            }

            if !error {
                // display the task listing
                tasks.list_for_today();
            }
        }
        // Mark a task as done for the day
        Opt::Done { index } => {
            // check that the values are in range
            let num_tasks = tasks.task_iter().count();
            let max_index = num_tasks - 1;
            let mut error = false;

            if index > max_index {
                error = true;
                println!(
                    "error: index out of range, values should be between 0 and {}",
                    max_index
                );
            } else {
                match tasks.task_iter_mut().nth(index).unwrap().mark_complete() {
                    Err(e) => {
                        error = true;
                        match e {
                            TaskError::AlreadyCompleted => {
                                println!("error: task was already completed today");
                            }
                            _ => {
                                println!(
                                    "error: unknown error occurred while marking task complete"
                                );
                            }
                        }
                    }
                    Ok(_) => (),
                }
            }

            if !error {
                println!();
                println!(
                    "Completed \"{}\"",
                    tasks
                        .task_iter_mut()
                        .nth(index)
                        .unwrap()
                        .details()
                        .unwrap()
                        .description()
                );
                println!();

                tasks.list_for_today();
            }
        }
    };

    // At this point the `TaskListing` should be in its finalized form

    // Create a serializer (note: it has to be done this way to be able to specify struct_names =
    // true)
    let ron_config = ron::ser::PrettyConfig {
        ..Default::default()
    };
    let mut serializer = ron::ser::Serializer::new(Some(ron_config), true);

    // Run the serializer on our task data, get back a string
    // TODO: maybe the file should have a checksum so that we can detect corruption from manual
    // editing
    match tasks.serialize(&mut serializer) {
        Err(e) => match e {
            ron::ser::Error::Message(s) => panic!("RON serialization error: {}", s),
        },
        Ok(_) => {}
    }
    let serialized = serializer.into_output_string();

    // Write the serialized data to chain's data folder
    let mut tasks_path = data_path;
    tasks_path.push(TASK_FILE);
    let mut tasks_file = match OpenOptions::new()
        .write(true)
        .truncate(true) // truncate, or else the file will be appended to
        .open(&tasks_path)
    {
        Err(e) => panic!("couldn't open {}: {}", TASK_FILE, e.description()),
        Ok(file) => file,
    };
    match tasks_file.write_all(serialized.as_bytes()) {
        Err(e) => panic!("couldn't write to {}: {}", TASK_FILE, e.description()),
        Ok(_) => (),
    }

    // All done!
}
