use crate::statement::Statement;

#[derive(Debug)]
pub(crate) struct Proc {
    pub signature: Signature,
    pub body: Statement,
}

#[derive(Debug)]
pub(crate) enum Signature {
    Custom { name: String, params: Vec<String> },
    WhenFlagClicked,
}
