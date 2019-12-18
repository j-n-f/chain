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

use structs::{Task, TaskListing};

/// Configuration for `structopt`
#[derive(StructOpt, Debug)]
#[structopt(name = "chain", about = "daily task tracking")]
enum Opt {
    #[structopt(name = "new", about = "create a new task")]
    New { description: String },
    #[structopt(name = "today", about = "view task status for today")]
    Today,
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
            // Calculate some field widths
            let indent_size = 4;
            let description_width = ((tasks.task_iter().fold(0, |max, task| {
                let curr_len = task.details().unwrap().description().chars().count();
                if max > curr_len {
                    max
                } else {
                    curr_len
                }
            }) / indent_size)
                + 1)
                * indent_size;
            let id_width = ((tasks.task_iter().count().to_string().chars().count() / 4) + 1) * 4;

            // Display header
            println!();
            println!("Task status for {}", Local::today().format("%F"));
            println!();

            // Display tasks
            for (n, task) in tasks.task_iter().enumerate() {
                // Check box
                if task.completed_today().is_some() {
                    print!("{:<4}", "[x]");
                } else {
                    print!("{:<4}", "[ ]")
                }

                // Numeric ID (used for "order" subcommand
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
                if n == 0 {
                    print!("(next)")
                }

                println!();
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
