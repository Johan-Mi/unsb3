use crate::statement::Statement;

pub(crate) struct Proc {
    pub params: Vec<String>,
    pub body: Statement,
}
