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

    pub(crate) fn to_num(&self) -> f32 {
        todo!()
    }
}
