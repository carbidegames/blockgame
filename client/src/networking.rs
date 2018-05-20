use {
    std::net::{SocketAddr},

    nalgebra::{Vector2, Point3},
    slog::{Logger},
    udpcon::{Peer, Event},

    blockgame_server::{self, message::{ClientMessage, PlayerFrame, ServerMessage}},
};


pub struct Connection {
    server: SocketAddr,
    peer: Option<Peer>,
    connected: bool,
}

impl Connection {
    pub fn new() -> Self {
        let server = "127.0.0.1:25566".parse().unwrap();
        let mut peer = Peer::start(None, blockgame_server::PROTOCOL);
        peer.connect(server);

        Connection {
            server,
            peer: Some(peer),
            connected: false,
        }
    }

    pub fn stop(&mut self) {
        self.peer.take().unwrap().stop();
        self.connected = false;
    }

    pub fn update(&mut self, log: &Logger, player_position: &mut Point3<f32>) {
        if let Some(ref mut peer) = self.peer {
            for event in peer.poll() {
                match event {
                    Event::NewPeer { address } => {
                        info!(log, "Server Connected: {}", address);
                        self.connected = true;
                    },
                    Event::PeerTimedOut { address } => {
                        info!(log, "Server Disconnected: {}", address);
                        self.connected = false;
                    },
                    Event::Message { source: _source, data } => {
                        // TODO: Disconnect from servers sending invalid packets
                        if let Some(message) = ServerMessage::deserialize(&data) {
                            match message {
                                ServerMessage::PlayerPosition(new_player_position) =>
                                    *player_position = new_player_position.position,
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn send_input(&mut self, input: Vector2<f32>) {
        if !self.connected {
            return
        }

        let message = ClientMessage::PlayerFrame(PlayerFrame { input });
        self.peer.as_mut().unwrap().send(self.server, message.serialize()).unwrap();
    }
}
