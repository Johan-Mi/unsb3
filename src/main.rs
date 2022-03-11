#![warn(clippy::unwrap_used, clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use crate::vm::VM;
use std::fs::File;

mod deser;
mod expr;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() {
    let path = "project.sb3";

    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("IO error: {err}");
            return;
        }
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(zip) => zip,
        Err(err) => {
            eprintln!("Zip error: {err}");
            return;
        }
    };

    let project_json = match archive.by_name("project.json") {
        Ok(zip) => zip,
        Err(err) => {
            eprintln!("Zip error: {err}");
            return;
        }
    };

    let vm: VM = match serde_json::from_reader(project_json) {
        Ok(vm) => vm,
        Err(err) => {
            eprintln!("Deserialization error: {err}");
            return;
        }
    };

    match vm.run() {
        Ok(()) => println!("Ran successfully"),
        Err(err) => eprintln!("VM error: {err}"),
    }
}
