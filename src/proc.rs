use crate::statement::Statement;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct Proc {
    pub signature: Signature,
    pub body: Statement,
}

#[derive(Debug)]
pub(crate) enum Signature {
    Custom {
        name: String,
        arg_names_by_id: HashMap<String, String>,
    },
    WhenFlagClicked,
    WhenBroadcastReceived {
        broadcast_name: String,
    },
}

impl Proc {
    pub fn name_is(&self, name: &str) -> bool {
        matches!(&self.signature,
            Signature::Custom { name: my_name, .. } if my_name == name)
    }

    pub fn is_the_broadcast(&self, name: &str) -> bool {
        matches!(&self.signature,
            Signature::WhenBroadcastReceived { broadcast_name, .. }
                if broadcast_name == name)
    }
}
