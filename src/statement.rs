use crate::{expr::Expr, field::Field};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) enum Statement {
    Builtin {
        opcode: String,
        inputs: HashMap<String, Expr>,
        fields: HashMap<String, Field>,
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
        counter: String,
        times: Expr,
        body: Box<Statement>,
    },
}
