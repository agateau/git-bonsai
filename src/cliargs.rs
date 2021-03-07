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
use structopt::StructOpt;

/// Keep a git repository clean and tidy.
#[derive(StructOpt)]
pub struct CliArgs {
    /// Branches to protect from suppression (in addition to master)
    #[structopt(short = "x", long)]
    pub excluded: Vec<String>,

    /// Do not fetch changes
    #[structopt(long = "no-fetch")]
    pub no_fetch: bool,

    /// Do not ask for confirmation
    #[structopt(short = "y", long = "yes")]
    pub yes: bool,
}
