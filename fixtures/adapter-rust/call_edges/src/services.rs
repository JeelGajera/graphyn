pub struct UserService {
    pub name: String,
}

impl UserService {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn handle(&self) -> &str {
        &self.name
    }
}

pub struct Wrapper(pub u32);

pub enum Outcome {
    Ready(String),
    Idle,
}

pub fn format_name(first: &str, last: &str) -> String {
    format!("{first} {last}")
}

pub fn unused_helper() {}
