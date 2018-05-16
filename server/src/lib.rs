#[macro_use] extern crate slog;
extern crate udpcon;

use {
    std::{thread, time::{Duration}},
    slog::{Logger},
    udpcon::{Peer, PeerMode},
};

pub const PROTOCOL: &str = concat!("blockgame-", env!("CARGO_PKG_VERSION"));

pub fn run(log: &Logger) {
    info!(log, "Starting Server");

    let address = "0.0.0.0:25566".parse().unwrap();
    let server = Peer::start(PeerMode::Server { address }, PROTOCOL);

    loop {
        while let Some(data) = server.try_recv() {
            println!("Data {:?} from {:?}", data.0, data.1);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
