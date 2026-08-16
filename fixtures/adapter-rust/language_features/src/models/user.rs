use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPayload {
    pub user_id: String,
    pub email: String,
    pub timestamp: String,
}

pub trait Identify {
    fn identity(&self) -> String;
}

impl Identify for UserPayload {
    fn identity(&self) -> String {
        self.user_id.clone()
    }
}

pub enum Status {
    Active,
    Suspended,
}
