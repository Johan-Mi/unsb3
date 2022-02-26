use crate::statement::Statement;

pub(crate) struct Proc {
    params: Vec<String>,
    pub body: Statement,
}
