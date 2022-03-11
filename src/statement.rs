use crate::expr::Expr;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Statement {
    Builtin {
        opcode: String,
        inputs: HashMap<String, Expr>,
    },
    Do(Vec<Statement>),
    IfElse {
        condition: Expr,
        if_true: Box<Statement>,
        if_false: Box<Statement>,
    },
    Repeat {
        times: Expr,
        body: Box<Statement>,
    },
    Forever {
        body: Box<Statement>,
    },
    Until {
        condition: Expr,
        body: Box<Statement>,
    },
    While {
        condition: Expr,
        body: Box<Statement>,
    },
    For {
        counter_id: String,
        times: Expr,
        body: Box<Statement>,
    },
    ProcCall {
        proccode: String,
        args: HashMap<String, Expr>,
    },
    DeleteAllOfList {
        list_id: String,
    },
    DeleteOfList {
        list_id: String,
        index: Expr,
    },
    AddToList {
        list_id: String,
        item: Expr,
    },
    ReplaceItemOfList {
        list_id: String,
        index: Expr,
        item: Expr,
    },
    SetVariable {
        var_id: String,
        value: Expr,
    },
    ChangeVariableBy {
        var_id: String,
        value: Expr,
    },
    StopAll,
    StopThisScript,
}
