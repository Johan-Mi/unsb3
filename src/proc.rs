use crate::statement::Statement;
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Custom {
    pub arg_names_by_id: HashMap<SmolStr, SmolStr>,
    pub body: Statement,
}

#[derive(Debug)]
pub struct Procs {
    pub when_flag_clicked: Vec<Statement>,
    pub custom: HashMap<String, Custom>,
    pub broadcasts: HashMap<String, Vec<Statement>>,
}
