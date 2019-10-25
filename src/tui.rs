use std::io::{stdin,stdout,Write};
use ansi_term::Color::{Blue,Red,Yellow};

pub fn read_line() -> String {
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn confirm(msg: &str) -> bool {
    print!("{} ", msg);
    stdout().flush().expect("Failed to flush");

    let input = read_line();

    input == "y" || input == "Y"
}

pub fn log_warning(msg: &str) {
    println!("{}", Yellow.paint(format!("Warning: {}", msg)));
}

pub fn log_error(msg: &str) {
    println!("{}", Red.paint(format!("Error: {}", msg)));
}

pub fn log_info(msg: &str) {
    println!("{}", Blue.paint(format!("Info: {}", msg)));
}
