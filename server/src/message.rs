use {
    nalgebra::{Vector2, Point3},
    bincode,
};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    PlayerFrame(PlayerFrame),
}

impl ClientMessage {
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


#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    PlayerUpdate(PlayerUpdate),
}

impl ServerMessage {
    pub fn deserialize(data: &Vec<u8>) -> Option<Self> {
        bincode::deserialize(&data).ok()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerUpdate {
    pub position: Point3<f32>,
}
