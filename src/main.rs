#![forbid(unsafe_code)]
#![warn(clippy::unwrap_used, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use crate::vm::VM;
use std::{fs::File, process::ExitCode};

mod deser;
mod expr;
mod proc;
mod sprite;
mod statement;
mod vm;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

fn real_main() -> Result<(), ()> {
    let path = std::env::args().nth(1);
    let path = path.as_deref().unwrap_or("project.sb3");

    let file = File::open(path).map_err(|err| eprintln!("IO error: {err}"))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| eprintln!("Zip error: {err}"))?;

    let project_json = archive
        .by_name("project.json")
        .map_err(|err| eprintln!("Zip error: {err}"))?;

    let vm: VM = serde_json::from_reader(project_json)
        .map_err(|err| eprintln!("Deserialization error: {err}"))?;

    vm.run().map_err(|err| eprintln!("VM error: {err}"))
}
