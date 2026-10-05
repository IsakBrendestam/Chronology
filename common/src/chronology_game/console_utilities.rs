//! This module contains macros that simplifies
//! the interaction with the termanl. Reducing
//! some expressions that would otherwise be
//! verbose.

/// Flushes any data in [`std::io::stdout`] to
/// terminal.
#[macro_export]
macro_rules! flush {
    () => {
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    };
}

/// Clears the terminal of any content.
#[macro_export]
macro_rules! clear {
    () => {
        print!("\x1b[2J\x1b[1;1H");
        flush!();
    };
}

/// Sets the terminal to write any proceeding
/// text in colour green.
#[macro_export]
macro_rules! set_green {
    () => {
        print!("\x1b[32m");
    };
}

/// Sets the terminal to write any proceeding
/// text in colour blue.
#[macro_export]
macro_rules! set_blue {
    () => {
        print!("\x1b[34m");
    };
}

/// Sets the terminal to write any proceeding
/// text in colour red.
#[macro_export]
macro_rules! set_red {
    () => {
        print!("\x1b[31m");
    };
}

/// Resets whatever colour the terminal is
/// currently set to write in.
#[macro_export]
macro_rules! reset_color {
    () => {
        print!("\x1b[0m");
    };
}
