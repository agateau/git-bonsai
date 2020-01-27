use ansi_term::Color::{Blue,Red,Yellow};

use dialoguer::{theme::ColorfulTheme, Checkboxes};

pub fn log_warning(msg: &str) {
    println!("{}", Yellow.paint(format!("Warning: {}", msg)));
}

pub fn log_error(msg: &str) {
    println!("{}", Red.paint(format!("Error: {}", msg)));
}

pub fn log_info(msg: &str) {
    println!("{}", Blue.paint(format!("Info: {}", msg)));
}

pub fn select(msg: &str, items: &Vec<String>) -> Vec<String> {
    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .items(&items[..])
        .interact()
        .unwrap();

    selections.iter().map(|&x| items[x].clone()).collect::<Vec<String>>()
}
