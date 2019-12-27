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

/// Represents an operation to perform on a TaskListing
pub enum TaskOperation {
    Add {
        /// Description of the task being added
        description: String,
    },
    MarkComplete {
        /// Index of task to mark complete
        task_index: usize,
        /// Optional remark on task completion
        remark: Option<String>,
    },
    Reorder {
        /// Index of task being moved
        from: usize,
        /// Index where `from` will be inserted, moving all tasks at this index and higher to a
        /// higher index
        to: usize,
    },
}
