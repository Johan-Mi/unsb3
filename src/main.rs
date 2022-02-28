use crate::vm::VM;
use std::{fs::File, io::BufReader};

mod deser;
mod expr;
mod field;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() {
    let path = "project.json";
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("IO error: {err}");
            return;
        }
    };
    let reader = BufReader::new(file);
    let vm: VM = match serde_json::from_reader(reader) {
        Ok(vm) => vm,
        Err(err) => {
            eprintln!("Deserialization error: {err}");
            return;
        }
    };
    println!("{vm:#?}");

    match vm.run() {
        Ok(()) => println!("Ran successfully"),
        Err(err) => eprintln!("VM error: {err}"),
    }
}
