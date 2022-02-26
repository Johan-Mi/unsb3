use crate::expr::Expr;

pub(crate) enum Statement {
    Call {
        proc_name: String,
        args: Vec<Expr>,
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
        counter: Expr,
        times: Expr,
        body: Box<Statement>,
    },
}
