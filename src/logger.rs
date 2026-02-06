// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{fs::OpenOptions, path::Path};

use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode, WriteLogger};

pub fn setup_file_logger(log_path: &Path) {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap_or_else(|err| {
            eprintln!("Failed to create {}: {}", log_path.display(), err);
            ::std::process::exit(1);
        });
    let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), file);
}

// Used by createsandbox
#[allow(dead_code)]
pub fn setup_stderr_logger() {
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    );
}
