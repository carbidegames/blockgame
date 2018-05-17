#[macro_use] extern crate slog;
extern crate udpcon;

use {
    std::{thread, time::{Duration}},
    slog::{Logger},
    udpcon::{Peer},
};

pub const PROTOCOL: &str = concat!("blockgame-", env!("CARGO_PKG_VERSION"));

pub fn run(log: &Logger) {
    info!(log, "Starting Server");

    let address = "0.0.0.0:25566".parse().unwrap();
    let mut server = Peer::start(Some(address), PROTOCOL);

    loop {
        for event in server.poll() {
            info!(log, "Network Event {:?}", event);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
