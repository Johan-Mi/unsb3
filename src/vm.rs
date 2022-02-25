use crate::sprite::Sprite;
use std::collections::HashMap;

pub(crate) struct VM {
    pub sprites: HashMap<String, Sprite>,
}

type VMResult<T> = Result<T, String>; // TODO: Proper error type

impl VM {
    pub fn run(&self) -> VMResult<()> {
        println!("Running vm...");
        Ok(())
    }
}
