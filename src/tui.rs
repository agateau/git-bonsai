use console::style;

use dialoguer::{theme::ColorfulTheme, Checkboxes};

pub fn log_warning(msg: &str) {
    println!("{}", style(format!("Warning: {}", msg)).yellow());
}

pub fn log_error(msg: &str) {
    println!("{}", style(format!("Error: {}", msg)).red());
}

pub fn log_info(msg: &str) {
    println!("{}", style(format!("Info: {}", msg)).blue());
}

pub fn select(msg: &str, items: &Vec<String>) -> Vec<String> {
    let checked_items : Vec<(String, bool)> = items.iter()
        .map(|x| (x.clone(), true)).collect();

    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt(msg)
        .items_checked(&checked_items[..])
        .interact()
        .unwrap();

    selections.iter().map(|&x| items[x].clone()).collect::<Vec<String>>()
}
