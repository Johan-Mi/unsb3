pub(crate) enum Expr {
    Lit(Value),
    Sym(String),
    Call { func_name: String, args: Vec<Expr> },
}

pub(crate) enum Value {
    Num(f32),
    Str(String),
    Bool(bool),
}
