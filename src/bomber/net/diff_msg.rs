use crate::bomber::gen::item::InteractiveItem;
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