use std::{cmp, collections::HashMap, fmt};

#[derive(Debug)]
pub(crate) enum Expr {
    Lit(Value),
    GetVar {
        var_id: String,
    },
    ProcArgStringNumber {
        name: String,
    },
    ItemOfList {
        list_id: String,
        index: Box<Expr>,
    },
    LengthOfList {
        list_id: String,
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
        inputs: HashMap<String, Expr>,
    },
}

#[derive(Clone)]
pub(crate) enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(num) => fmt::Debug::fmt(num, f),
            Value::Str(s) => fmt::Debug::fmt(s, f),
            Value::Bool(b) => fmt::Debug::fmt(b, f),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Str(String::new())
    }
}

pub(crate) enum Index {
    Nth(usize),
    Last,
}

impl Value {
    pub(crate) fn to_bool(&self) -> bool {
        match self {
            Value::Num(num) => *num != 0.0 && !num.is_nan(),
            Value::Str(s) => {
                !s.is_empty() && s != "0" && !s.eq_ignore_ascii_case("false")
            }
            Value::Bool(b) => *b,
        }
    }

    pub(crate) fn try_to_num(&self) -> Option<f64> {
        match self {
            Value::Num(num) if num.is_nan() => None,
            Value::Num(num) => Some(*num),
            Value::Str(s) => try_str_to_num(s),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }

    pub(crate) fn to_num(&self) -> f64 {
        self.try_to_num().unwrap_or(0.0)
    }

    pub(crate) fn to_index(&self) -> Option<Index> {
        // TODO: Handle "all", "random" and "any"
        match self {
            Value::Str(s) if s == "last" => Some(Index::Last),
            _ => self
                .try_to_num()
                .and_then(|n| (n as usize).checked_sub(1).map(Index::Nth)),
        }
    }

    pub(crate) fn compare(&self, other: &Self) -> cmp::Ordering {
        if let (Some(lhsn), Some(rhsn)) =
            (self.try_to_num(), other.try_to_num())
        {
            match lhsn.partial_cmp(&rhsn) {
                Some(ord) => ord,
                None => panic!("could not compare {lhsn} with {rhsn}"),
            }
        } else {
            // TODO: Do this without allocating new strings
            self.to_string()
                .to_lowercase()
                .cmp(&other.to_string().to_lowercase())
        }
    }
}

pub(crate) fn try_str_to_num(s: &str) -> Option<f64> {
    match s.trim() {
        "Infinity" | "+Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        "inf" | "+inf" | "-inf" => None,
        s => s.parse().ok().filter(|n: &f64| !n.is_nan()),
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(num) => number_to_string(*num).fmt(f),
            Value::Str(s) => s.fmt(f),
            Value::Bool(b) => b.fmt(f),
        }
    }
}

fn number_to_string(num: f64) -> String {
    // FIXME: Rust does not format floats the same way as JavaScript.
    if num == f64::INFINITY {
        "Infinity".to_owned()
    } else if num == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else {
        num.to_string()
    }
}
