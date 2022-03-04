use crate::statement::Statement;

#[derive(Debug)]
pub(crate) struct Proc {
    pub signature: Signature,
    pub body: Statement,
}

#[derive(Debug)]
pub(crate) enum Signature {
    Custom {
        name: String,
        param_ids: Vec<String>,
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
}
