use crate::statement::Statement;

pub(crate) struct Proc {
    params: Vec<String>,
    body: Statement,
}
