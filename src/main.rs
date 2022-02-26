use crate::{
    expr::{Expr, Value},
    proc::Proc,
    sprite::Sprite,
    statement::Statement,
    vm::VM,
};
use std::collections::HashMap;

mod expr;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() {
    let mut sprites = HashMap::new();
    sprites.insert(
        "sprite-1".to_owned(),
        Sprite {
            procs: vec![Proc {
                params: Vec::new(),
                body: Statement::Repeat {
                    times: Expr::Lit(Value::Num(10.0)),
                    body: Box::new(Statement::Call {
                        proc_name: "print".to_owned(),
                        args: vec![Expr::Lit(Value::Str(
                            "Hello, world!".to_owned(),
                        ))],
                    }),
                },
            }],
        },
    );

    let vm = VM { sprites };

    match vm.run() {
        Ok(()) => println!("Ran successfully"),
        Err(err) => eprintln!("Error: {err}"),
    }
}
