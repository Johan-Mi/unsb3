use std::fmt;

pub(crate) enum Expr {
    Lit(Value),
    Sym(String),
    Call { func_name: String, args: Vec<Expr> },
}

#[derive(Clone)]
pub(crate) enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl Value {
    pub(crate) fn to_bool(&self) -> bool {
        match self {
            Value::Num(num) => *num != 0.0 && !num.is_nan(),
            Value::Str(s) => {
                s != "" && s != "0" && !s.eq_ignore_ascii_case("false")
            }
            Value::Bool(b) => *b,
        }
    }

    pub(crate) fn to_num(&self) -> f64 {
        match self {
            Value::Num(num) => *num,
            Value::Str(s) => match s.trim() {
                "Infinity" | "+Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "inf" | "+inf" | "-inf" => 0.0,
                s => s.parse().unwrap_or(0.0),
            },
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
        }
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
