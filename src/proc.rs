use crate::statement::Statement;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Proc {
    pub signature: Signature,
    pub body: Statement,
}

#[derive(Debug)]
pub enum Signature {
    WhenFlagClicked,
    WhenBroadcastReceived { broadcast_name: String },
}

impl Proc {
    pub fn is_the_broadcast(&self, name: &str) -> bool {
        matches!(&self.signature,
            Signature::WhenBroadcastReceived { broadcast_name, .. }
                if broadcast_name == name)
    }
}

#[derive(Debug)]
pub struct Custom {
    pub arg_names_by_id: HashMap<String, String>,
    pub body: Statement,
}

#[derive(Debug)]
pub struct Procs {
    pub normal: Vec<Proc>,
    pub custom: HashMap<String, Custom>,
}
