use crate::statement::Statement;

#[derive(Debug)]
pub(crate) struct Proc {
    pub params: Vec<String>,
    pub body: Statement,
}
