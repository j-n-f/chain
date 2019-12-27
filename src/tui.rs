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
use pancurses::*;
use pancurses::{
    curs_set, endwin, init_pair, initscr, noecho, resize_term, start_color, use_default_colors,
    Input, Window, A_BOLD, A_REVERSE,
};

use super::structs::TaskListing;

enum UiMode {
    Listing {
        /// None if no task is selected (i.e. none exist), otherwise the index into the TaskListing
        /// for the currently highlighted task
        task_index: Option<usize>,
        /// None if no rows need cleaning up, otherwise the index of the row that needs to have
        /// active task styles reverted
        prev_index: Option<usize>,
        /// Index of task which is currently at top of listing
        scroll_pos: usize,
    },
}

struct Ui {
    window: Option<Window>,
    mode: UiMode,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            window: None,
            mode: UiMode::Listing {
                task_index: None,
                prev_index: None,
                scroll_pos: 0,
            },
        }
    }
}

impl Ui {
    fn window(&self) -> &Window {
        self.window.as_ref().unwrap()
    }
}

fn render_listing(ui: &mut Ui, tasks: &TaskListing) {
    let description_width = 40;
    let w = ui.window();

    // Header + calendar dates
    w.mvaddstr(2, 0, "Task");
    w.mvchgat(2, 0, 40, A_BOLD | A_UNDERLINE, 0);

    let calendar_pad = 2;
    let cal_width = w.get_max_x() - 0 - (description_width + calendar_pad);
    let cal_n_days = cal_width / 4;

    let mut today = Utc::now().with_timezone(&Local).date();
    for _n in 0..cal_n_days - 1 {
        today = today.pred();
    }

    let start = today.clone();

    for n in 0..cal_n_days {
        let col = description_width + calendar_pad + (4 * n);

        if n == 0 || today.day() == 1 {
            w.mvaddstr(1, col - 1, " ");
            w.mvaddstr(1, col, format!("{}", today.format("%h")));
        } else {
            w.mvaddstr(1, col, "----");
        }

        w.mvaddstr(2, col, format!("{:<02}", today.day()));
        w.mvchgat(2, col, 3, A_BOLD, 0);
        today = today.succ();
    }
    today = today.pred();

    // Task listing
    // TODO: stop if more tasks than can fit on screen
    // TODO: scrolling
    // TODO: don't show "x" if task didn't exist yet
    if let Some(index) = match ui.mode {
        UiMode::Listing { prev_index, .. } => prev_index,
    } {
        w.mvchgat((3 + index) as i32, 0, w.get_max_x(), A_NORMAL, 0);
    }
    for (n, task) in tasks.task_iter().enumerate() {
        let description = task.description();
        let mut description_fmt = description.clone();

        let active_task: bool = match ui.mode {
            UiMode::Listing { task_index, .. } => {
                if n == task_index.unwrap() {
                    true
                } else {
                    false
                }
            }
        };

        if description.chars().count() > description_width as usize {
            description_fmt.truncate(description_width as usize - 3);
            description_fmt.push_str("...");
        }

        w.mvaddstr((3 + n) as i32, 0, description_fmt);
        if active_task {
            w.mvchgat((3 + n) as i32, 0, w.get_max_x(), A_UNDERLINE, 0);
        }

        // render completion status
        let mut day = start.clone();
        let mut day_n = 0;
        while day != today.succ() {
            let col = description_width + calendar_pad + (4 * day_n);
            let style = if active_task { A_UNDERLINE } else { 0 };
            let is_today = day == today;
            if task.completed_on(day) {
                init_pair(1, COLOR_GREEN, -1);
                w.mvaddstr((3 + n) as i32, col, "o---");
                w.mvchgat((3 + n) as i32, col, 4, style, 1);
            } else {
                init_pair(2, COLOR_RED, -1);
                init_pair(3, COLOR_YELLOW, -1);
                if is_today {
                    w.mvaddstr((3 + n) as i32, col, "?   ");
                    w.mvchgat((3 + n) as i32, col, 4, style | A_BLINK, 3);
                } else {
                    w.mvaddstr((3 + n) as i32, col, "x   ");
                    w.mvchgat((3 + n) as i32, col, 4, style, 2);
                }
            }
            day_n += 1;
            day = day.succ();
        }
    }

    // dummy: keyboard hints
    ui.window().mvaddstr(
        ui.window().get_max_y() - 2,
        0,
        "[space] complete - [enter] complete with remark - [r] add remark",
    );
}

/// returns `true` for as long as the loop should keep running
// TODO: this should yeild an optional operation to apply to the `TaskListing`
fn input_and_render(ui: &mut Ui, tasks: &TaskListing) -> bool {
    let task_count = tasks.task_iter().count();
    let max_task_index: usize = if task_count > 0 { task_count - 1 } else { 0 };

    // Title bar
    ui.window().mvaddstr(0, 0, "chain (v0.1.0)");
    ui.window()
        .mvchgat(0, 0, ui.window().get_max_yx().1, A_UNDERLINE, 0);

    // Debug: window dimensions
    let dim_string = format!("{} x {}", ui.window().get_max_x(), ui.window().get_max_y());
    ui.window().mvaddstr(
        ui.window().get_max_y() - 1,
        ui.window().get_max_x() - dim_string.chars().count() as i32,
        dim_string,
    );
    // Mode-specific rendering
    match ui.mode {
        UiMode::Listing { .. } => {
            render_listing(ui, tasks);
        }
    }

    // Bottom line is entry bar
    ui.window().mvchgat(
        ui.window().get_max_y() - 1,
        0,
        ui.window().get_max_x(),
        A_REVERSE,
        0,
    );

    // Handle input
    if let Some(input) = ui.window().getch() {
        ui.window().mvaddstr(10, 0, " ".repeat(40));
        match input {
            Input::KeyUp => match &mut ui.mode {
                UiMode::Listing {
                    task_index,
                    prev_index,
                    ..
                } => {
                    if let Some(index) = task_index {
                        if *index > 0 {
                            *prev_index = Some(*index);
                            *task_index = Some(*index - 1);
                        }
                    }
                }
            },
            Input::KeyDown => match &mut ui.mode {
                UiMode::Listing {
                    task_index,
                    prev_index,
                    ..
                } => {
                    if let Some(index) = task_index {
                        if *index < max_task_index {
                            *prev_index = Some(*index);
                            *task_index = Some(*index + 1);
                        }
                    }
                }
            },
            Input::Unknown(n) => {
                ui.window().mvaddstr(10, 0, format!("UK {:?}", n));
            }
            Input::KeyResize => {
                resize_term(0, 0);
                ui.window().clear();
            }
            _ => {
                //w.mvaddstr(10, 0, format!("{:?}", input));
            }
        };
    }

    // TODO: show month names A_DIM
    // TODO: display completion time
    // TODO: display (next) [I could just have finished and queued as A_DIM]
    // TODO: never show a date earlier than the earliest completion in the whole database

    ui.window().refresh();

    true
}

pub fn run(tasks: &mut TaskListing) {
    // Initialize the window
    let w = initscr();
    let mut ui: Ui = Ui::default();
    ui.window = Some(w);
    ui.window().keypad(true); //< makes it so that arrow/function keys are properly represented
    noecho();
    use_default_colors();
    start_color();
    set_blink(true);
    curs_set(0);

    ui.mode = if tasks.task_iter().count() > 0 {
        UiMode::Listing {
            task_index: Some(0),
            prev_index: None,
            scroll_pos: 0,
        }
    } else {
        UiMode::Listing {
            task_index: None,
            prev_index: None,
            scroll_pos: 0,
        }
    };

    while input_and_render(&mut ui, tasks) {}

    // Clean up the window and restore terminal state
    endwin();
}
