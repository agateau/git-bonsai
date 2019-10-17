use std::io::{stdin,stdout,Write};

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
    println!("Warning: {}", msg);
}

pub fn log_error(msg: &str) {
    println!("Error: {}", msg);
}
