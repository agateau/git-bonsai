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

mod cliargs;
mod gitsynctask;
mod logger;
mod model;
mod ratapp;
mod repositorymodel;
mod task;
mod ui;
mod worker;

use cliargs::CliArgs;

fn main() {
    let args = CliArgs::from_args();
    if let Some(log_path) = &args.log_path {
        logger::setup_file_logger(log_path);
    }
    log::info!("Start");
    let exit_code = ratapp::run(args, ".");
    log::info!("Stop");
    ::std::process::exit(exit_code);
}
