use crate::vm::VM;
use std::collections::HashMap;

mod expr;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() {
    let vm = VM {
        sprites: HashMap::new(),
    };

    match vm.run() {
        Ok(()) => println!("Ran successfully"),
        Err(err) => eprintln!("Error: {err}"),
    }
}
