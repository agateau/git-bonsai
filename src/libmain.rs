/*
 * Copyright 2021 Aurélien Gâteau <mail@agateau.com>
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
use crate::app::App;
use crate::appui::AppUi;
use crate::batchappui::BatchAppUi;
use crate::cliargs::CliArgs;
use crate::interactiveappui::InteractiveAppUi;

pub fn libmain(args: CliArgs, dir: &str) -> i32 {
    let ui: Box<dyn AppUi> = match args.yes {
        false => Box::new(InteractiveAppUi {}),
        true => Box::new(BatchAppUi {}),
    };
    let app = App::new(&args, &*ui, &dir);

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
