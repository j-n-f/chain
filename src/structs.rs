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
pub mod tasklisting;
pub use tasklisting::TaskListing;

pub mod taskoperation;
pub use taskoperation::TaskOperation;

pub mod task;
pub use task::{Completion as TaskCompletion, Task, TaskDetails, TaskError};
