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
    Call {
        opcode: String,
        inputs: HashMap<String, Expr>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl Default for Value {
    fn default() -> Self {
        Value::Str(String::new())
    }
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
            Value::Num(num) => Some(*num),
            Value::Str(s) => try_str_to_num(s),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }

    pub(crate) fn to_num(&self) -> f64 {
        self.try_to_num().unwrap_or(0.0)
    }

    pub(crate) fn to_index(&self) -> usize {
        (self.to_num() - 1.0) as usize
    }

    pub(crate) fn compare(&self, other: &Self) -> cmp::Ordering {
        if let (Some(lhsn), Some(rhsn)) =
            (self.try_to_num(), other.try_to_num())
        {
            lhsn.partial_cmp(&rhsn).unwrap()
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
        s => s.parse().ok(),
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
    num.to_string()
}
