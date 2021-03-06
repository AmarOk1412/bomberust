use crate::bomber::gen::item::InteractiveItem;
use crate::bomber::gen::utils::SquareType;
use rmps::Serializer;
use serde::Serialize;

pub trait SerializedEvent {
    fn to_vec(&self) -> Vec<u8>;
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayerMove {
    pub msg_type: String,
    pub id: i32,
    pub x: f32,
    pub y: f32
}

impl SerializedEvent for PlayerMove {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BombMove {
    pub msg_type: String,
    pub old_x: f32,
    pub old_y: f32,
    pub x: f32,
    pub y: f32
}

impl SerializedEvent for BombMove {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayerPutBomb {
    pub msg_type: String,
    pub id: i32,
    pub x: usize,
    pub y: usize
}

impl SerializedEvent for PlayerPutBomb {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BombExplode {
    pub msg_type: String,
    pub w: u64,
    pub h: u64,
}

impl SerializedEvent for BombExplode {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayerDie {
    pub msg_type: String,
    pub id: u64,
}

impl SerializedEvent for PlayerDie {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlayerIdentity {
    pub msg_type: String,
    pub id: u64,
}

impl SerializedEvent for PlayerIdentity {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CreateItem {
    pub msg_type: String,
    pub item: Option<InteractiveItem>,
    pub w: u64,
    pub h: u64,
}

impl SerializedEvent for CreateItem {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UpdateSquare {
    pub msg_type: String,
    pub square: SquareType,
    pub x: u64,
    pub y: u64,
}

impl SerializedEvent for UpdateSquare {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DestroyItem {
    pub msg_type: String,
    pub w: u64,
    pub h: u64,
}

impl SerializedEvent for DestroyItem {
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        buf
    }
}