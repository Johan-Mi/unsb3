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
        index: Box<Self>,
    },
    LengthOfList {
        list_id: SmolStr,
    },
    Abs(Box<Self>),
    Floor(Box<Self>),
    Ceiling(Box<Self>),
    Sqrt(Box<Self>),
    Sin(Box<Self>),
    Cos(Box<Self>),
    Tan(Box<Self>),
    Asin(Box<Self>),
    Acos(Box<Self>),
    Atan(Box<Self>),
    Ln(Box<Self>),
    Log(Box<Self>),
    EExp(Box<Self>),
    TenExp(Box<Self>),
    Call {
        opcode: String,
        inputs: HashMap<SmolStr, Self>,
    },
}
