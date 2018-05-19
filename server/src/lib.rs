extern crate nalgebra;
#[macro_use] extern crate slog;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate bincode;
extern crate udpcon;

use {
    std::{thread, time::{Duration}},

    nalgebra::{Vector2},
    slog::{Logger},
    udpcon::{Peer, Event},
};

pub const PROTOCOL: &str = concat!("blockgame-", env!("CARGO_PKG_VERSION"));

pub fn run(log: &Logger) {
    info!(log, "Starting Server");

    let address = "0.0.0.0:25566".parse().unwrap();
    let mut peer = Peer::start(Some(address), PROTOCOL);

    loop {
        for event in peer.poll() {
            match event {
                Event::Message { source, data } => {
                    let message = Message::deserialize(&data);
                    info!(log, "Message: {:?} from {}", message, source)
                },
                Event::NewPeer { address } =>
                    info!(log, "Client Connected: {}", address),
                Event::PeerTimedOut { address } =>
                    info!(log, "Client Disconnected: {}", address),
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PlayerFrame(PlayerFrame),
}

impl Message {
    pub fn deserialize(data: &Vec<u8>) -> Option<Self> {
        bincode::deserialize(&data).ok()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerFrame {
    pub input: Vector2<f32>,
}
