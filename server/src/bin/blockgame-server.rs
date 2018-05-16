extern crate blockgame_server;
extern crate sloggers;

use {
    sloggers::{Build, terminal::{TerminalLoggerBuilder}, types::{Severity}},
};

fn main() {
    // Set up logging
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    let log = builder.build().unwrap();

    blockgame_server::run(&log);
}
