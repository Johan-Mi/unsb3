use crate::expr::Expr;
use ecow::EcoString;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Statement {
    Regular {
        opcode: EcoString,
        inputs: HashMap<EcoString, Expr>,
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
        counter_id: EcoString,
        times: Expr,
        body: Box<Self>,
    },
    ProcCall {
        proccode: String,
        args: HashMap<EcoString, Expr>,
    },
    DeleteAllOfList {
        list_id: EcoString,
    },
    DeleteOfList {
        list_id: EcoString,
        index: Expr,
    },
    AddToList {
        list_id: EcoString,
        item: Expr,
    },
    ReplaceItemOfList {
        list_id: EcoString,
        index: Expr,
        item: Expr,
    },
    SetVariable {
        var_id: EcoString,
        value: Expr,
    },
    ChangeVariableBy {
        var_id: EcoString,
        value: Expr,
    },
    StopAll,
    StopThisScript,
}
