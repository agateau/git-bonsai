/*
 * Copyright 2020 Aurélien Gâteau <mail@agateau.com>
 *
 * This file is part of git-bonsai.
 *
 * Git-bonsai is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */
use structopt::StructOpt;

mod app;
mod appui;
mod cliargs;
mod git;
mod interactiveappui;
mod tui;

use app::App;
use cliargs::CliArgs;

fn runapp() -> i32 {
    let args = CliArgs::from_args();

    let app = App::new(&args, ".");

    if !app.is_working_tree_clean() {
        return 1;
    }

    if !args.no_fetch {
        if let Err(x) = app.fetch_changes() {
            return x;
        }
    }

    if let Err(x) = app.update_tracking_branches() {
        return x;
    }

    if let Err(x) = app.remove_merged_branches() {
        return x;
    }
    0
}

fn main() {
    ::std::process::exit(runapp());
}
