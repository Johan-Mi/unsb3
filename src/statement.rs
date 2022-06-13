use crate::expr::Expr;
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Statement {
    Regular {
        opcode: SmolStr,
        inputs: HashMap<SmolStr, Expr>,
    },
    Do(Vec<Statement>),
    If {
        condition: Expr,
        if_true: Box<Statement>,
    },
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
        counter_id: SmolStr,
        times: Expr,
        body: Box<Statement>,
    },
    ProcCall {
        proccode: String,
        args: HashMap<SmolStr, Expr>,
    },
    DeleteAllOfList {
        list_id: SmolStr,
    },
    DeleteOfList {
        list_id: SmolStr,
        index: Expr,
    },
    AddToList {
        list_id: SmolStr,
        item: Expr,
    },
    ReplaceItemOfList {
        list_id: SmolStr,
        index: Expr,
        item: Expr,
    },
    SetVariable {
        var_id: SmolStr,
        value: Expr,
    },
    ChangeVariableBy {
        var_id: SmolStr,
        value: Expr,
    },
    StopAll,
    StopThisScript,
}
