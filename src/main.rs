use crate::{sprite::Sprite, vm::VM};
use std::collections::HashMap;

mod expr;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() {
    let mut sprites = HashMap::new();
    sprites.insert("sprite-1".to_owned(), Sprite { procs: Vec::new() });

    let vm = VM { sprites };

    match vm.run() {
        Ok(()) => println!("Ran successfully"),
        Err(err) => eprintln!("Error: {err}"),
    }
}
