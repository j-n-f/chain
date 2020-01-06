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

// TODO: this mixes operations on both `Task` and `TaskListing`, and should probably be cleaned up.

/// Represents an operation to perform on a TaskListing
#[derive(Debug)]
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
    AddRemark {
        /// Index of task to add remark to
        task_index: usize,
        /// Remark to add to task
        remark: String,
    },
    Reorder {
        /// Index of task being moved
        from: usize,
        /// Index where `from` will be inserted, moving all tasks at this index and higher to a
        /// higher index
        to: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::TaskOperation;
    use crate::structs::TaskError;
    use crate::structs::TaskListing;

    #[test]
    fn add_requires_description() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "".into(),
        };

        let result = list.handle_operation(&add);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::MissingDescription);
    }

    #[test]
    fn adds_with_description() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "non-zero length".into(),
        };

        let result = list.handle_operation(&add);
        assert!(result.is_ok());
    }

    #[test]
    fn reorder_no_tasks() {
        let mut list = TaskListing::new();

        let reorder = TaskOperation::Reorder { from: 0, to: 0 };

        let result = list.handle_operation(&reorder);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);
    }

    #[test]
    fn reorder_same_indexes() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let reorder = TaskOperation::Reorder { from: 0, to: 0 };

        let result = list.handle_operation(&reorder);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::RedundantMove);
    }

    #[test]
    fn reorder_same_indexes_no_tasks() {
        let mut list = TaskListing::new();

        let reorder = TaskOperation::Reorder { from: 0, to: 0 };

        let result = list.handle_operation(&reorder);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);
    }

    #[test]
    fn reorder_out_of_bounds() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let reorder = TaskOperation::Reorder { from: 0, to: 100 };

        let result = list.handle_operation(&reorder);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);
    }

    #[test]
    fn mark_complete_oob() {
        // Both with and without a remark
        let mut list = TaskListing::new();

        // No Remark
        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: None,
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);

        // With a remark
        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: Some("with a remark".into()),
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);
    }

    #[test]
    fn mark_complete_no_remark() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: None,
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_ok());
    }

    #[test]
    fn mark_complete_with_remark() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: Some("with some remark".into()),
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_ok());
    }

    #[test]
    fn mark_complete_twice() {
        // NOTE: this test could potentially fail if the two commands to mark the task complete
        // happen on opposite sides of the "midnight" boundary. Unlikely, but possible.
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: Some("with some remark".into()),
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_ok());

        let result = list.handle_operation(&complete);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::AlreadyCompleted);
    }

    #[test]
    fn remark_oob() {
        let mut list = TaskListing::new();

        let remark = TaskOperation::AddRemark {
            task_index: 0,
            remark: "with some remark".into(),
        };

        let result = list.handle_operation(&remark);
        assert!(result.is_err());
        assert!(result.unwrap_err() == TaskError::NotFound);
    }

    #[test]
    fn remark_on_incomplete() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let remark = TaskOperation::AddRemark {
            task_index: 0,
            remark: "with some remark".into(),
        };

        let result = list.handle_operation(&remark);
        assert!(result.is_ok());
    }

    #[test]
    fn remark_on_completed() {
        let mut list = TaskListing::new();

        let add = TaskOperation::Add {
            description: "first".into(),
        };
        assert!(list.handle_operation(&add).is_ok());

        let complete = TaskOperation::MarkComplete {
            task_index: 0,
            remark: Some("with some remark".into()),
        };

        let result = list.handle_operation(&complete);
        assert!(result.is_ok());

        let remark = TaskOperation::AddRemark {
            task_index: 0,
            remark: "with another remark".into(),
        };

        let result = list.handle_operation(&remark);
        assert!(result.is_ok());
    }
}
