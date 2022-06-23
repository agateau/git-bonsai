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

/**
 * This module contains "low-level" primitives to implement a text-based UI
 */
use console::style;

use dialoguer::{theme::ColorfulTheme, MultiSelect};

pub fn log_warning(msg: &str) {
    println!("{}", style(format!("Warning: {}", msg)).yellow());
}

pub fn log_error(msg: &str) {
    println!("{}", style(format!("Error: {}", msg)).red());
}

pub fn log_info(msg: &str) {
    println!("{}", style(format!("Info: {}", msg)).blue());
}

pub fn select(msg: &str, items: &[String]) -> Vec<usize> {
    let checked_items: Vec<(String, bool)> = items.iter().map(|x| (x.clone(), true)).collect();

    MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .items_checked(&checked_items[..])
        .interact()
        .unwrap()
}
