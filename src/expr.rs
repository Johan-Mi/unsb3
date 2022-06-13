use sb3_stuff::Value;
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Expr {
    Lit(Value),
    GetVar {
        var_id: SmolStr,
    },
    ProcArgStringNumber {
        name: SmolStr,
    },
    ItemOfList {
        list_id: SmolStr,
        index: Box<Expr>,
    },
    LengthOfList {
        list_id: SmolStr,
    },
    Abs(Box<Expr>),
    Floor(Box<Expr>),
    Ceiling(Box<Expr>),
    Sqrt(Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Tan(Box<Expr>),
    Asin(Box<Expr>),
    Acos(Box<Expr>),
    Atan(Box<Expr>),
    Ln(Box<Expr>),
    Log(Box<Expr>),
    EExp(Box<Expr>),
    TenExp(Box<Expr>),
    Call {
        opcode: String,
        inputs: HashMap<SmolStr, Expr>,
    },
}
