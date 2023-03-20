use crate::expr::Expr;
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Statement {
    Regular {
        opcode: SmolStr,
        inputs: HashMap<SmolStr, Expr>,
    },
    Do(Vec<Self>),
    If {
        condition: Expr,
        if_true: Box<Self>,
    },
    IfElse {
        condition: Expr,
        if_true: Box<Self>,
        if_false: Box<Self>,
    },
    Repeat {
        times: Expr,
        body: Box<Self>,
    },
    Forever {
        body: Box<Self>,
    },
    Until {
        condition: Expr,
        body: Box<Self>,
    },
    While {
        condition: Expr,
        body: Box<Self>,
    },
    For {
        counter_id: SmolStr,
        times: Expr,
        body: Box<Self>,
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
