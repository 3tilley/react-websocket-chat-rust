use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

pub type Users = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;
pub static NEXT_USERID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

#[derive(Serialize, Deserialize)]
pub struct Msg {
    name: String,
    pub uid: Option<usize>,
    message: String,
}

#[derive(Default)]
pub struct AppState {
    pub users: Mutex<HashMap<usize, UnboundedSender<Message>>>,
}