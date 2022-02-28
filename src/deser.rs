use crate::{
    expr::{str_to_num, Expr, Value},
    field::Field,
    proc::{Proc, Signature},
    statement::Statement,
};
use serde::Deserialize;
use serde_json::Value as Json;
use std::{borrow::Cow, collections::HashMap, fmt::Display};
use thiserror::Error;

pub(crate) struct DeCtx<'a> {
    blocks: HashMap<String, Block<'a>>,
}

#[derive(Debug, Error)]
pub(crate) enum DeError {
    #[error("{0}")]
    Custom(String),
    #[error("found no block with ID `{0}`")]
    NonExsistentID(String),
}

type DeResult<T> = Result<T, DeError>;

impl serde::de::Error for DeError {
    fn custom<T: Display>(msg: T) -> Self {
        DeError::Custom(msg.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Block<'a> {
    pub opcode: Cow<'a, str>,
    // pub parent: Option<String>,
    pub next: Option<String>,
    #[serde(default)]
    pub inputs: HashMap<String, Json>,
    #[serde(default)]
    pub fields: HashMap<String, Json>,
}

impl<'a> DeCtx<'a> {
    pub fn new(blocks: HashMap<String, Block<'a>>) -> Self {
        Self { blocks }
    }

    pub fn build_procs(&self) -> DeResult<Vec<Proc>> {
        self.blocks
            .values()
            .filter_map(|block| match &*block.opcode {
                "procedures_definition" => {
                    let next = block.next.as_ref()?;
                    // This should be a `try` block
                    Some((|| {
                        let body = self.build_statement(next)?;
                        let signature = todo!();
                        Ok(Proc { signature, body })
                    })())
                }
                "event_whenflagclicked" => {
                    let next = block.next.as_ref()?;
                    // This should be a `try` block
                    Some((|| {
                        let body = self.build_statement(next)?;
                        Ok(Proc {
                            signature: Signature::WhenFlagClicked,
                            body,
                        })
                    })())
                }
                _ => None,
            })
            .collect()
    }

    fn build_statement(&self, id: &str) -> DeResult<Statement> {
        let block = self.get(id)?;

        match &*block.opcode {
            "control_if" => {
                let condition = self.input(block, "CONDITION")?;
                let if_true = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_true = self.build_statement(if_true)?;
                Ok(Statement::IfElse {
                    condition,
                    if_true: Box::new(if_true),
                    if_false: Box::new(Statement::Do(Vec::new())),
                })
            }
            "control_if_else" => {
                let condition = self.input(block, "CONDITION")?;
                let if_true = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_true = self.build_statement(if_true)?;
                let if_false = block
                    .inputs
                    .get("SUBSTACK2")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_false = self.build_statement(if_false)?;
                Ok(Statement::IfElse {
                    condition,
                    if_true: Box::new(if_true),
                    if_false: Box::new(if_false),
                })
            }
            "control_repeat" => todo!(),
            "control_forever" => {
                let body = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let body = self.build_statement(body)?;
                Ok(Statement::Forever {
                    body: Box::new(body),
                })
            }
            "control_repeat_until" => todo!(),
            "control_while" => todo!(),
            "control_for_each" => {
                let body = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let body = self.build_statement(body)?;
                Ok(Statement::For {
                    counter: todo!(),
                    times: todo!(),
                    body: Box::new(body),
                })
            }
            opcode => {
                let inputs = block
                    .inputs
                    .iter()
                    .map(|(id, b)| Ok((id.clone(), self.build_expr(b)?)))
                    .collect::<Result<_, _>>()?;
                let fields = block
                    .fields
                    .iter()
                    .map(|(id, b)| Ok((id.clone(), self.build_field(b)?)))
                    .collect::<Result<_, _>>()?;
                Ok(Statement::Builtin {
                    opcode: opcode.to_string(),
                    inputs,
                    fields,
                })
            }
        }
    }

    fn build_expr(&self, json: &Json) -> DeResult<Expr> {
        let rep = get_rep(json).unwrap();
        match rep {
            Json::String(id) => self.build_funcall(id),
            Json::Array(arr) => match &arr[..] {
                [Json::Number(n), num]
                    if n == &serde_json::Number::from(10u32) =>
                {
                    let num = match num {
                        Json::Number(num) => f64::deserialize(num).unwrap(),
                        Json::String(s) => str_to_num(s),
                        _ => todo!(),
                    };
                    Ok(Expr::Lit(Value::Num(num)))
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    fn build_field(&self, json: &Json) -> DeResult<Field> {
        dbg!(json);
        todo!()
    }

    fn build_funcall(&self, id: &str) -> DeResult<Expr> {
        let block = self.get(id)?;

        // Field generation has to be done manually for each opcode that uses it
        if !block.fields.is_empty() {
            dbg!(block);
        }

        match &*block.opcode {
            opcode => {
                let inputs = block
                    .inputs
                    .iter()
                    .map(|(id, inp)| Ok((id.clone(), self.build_expr(inp)?)))
                    .collect::<Result<_, _>>()?;
                let fields = block
                    .fields
                    .iter()
                    .map(|(id, inp)| Ok((id.clone(), self.build_field(inp)?)))
                    .collect::<Result<_, _>>()?;
                Ok(Expr::Call {
                    opcode: opcode.to_string(),
                    inputs,
                    fields,
                })
            }
        }
    }

    fn input(&self, block: &Block, name: &str) -> DeResult<Expr> {
        self.build_expr(
            block
                .inputs
                .get(name)
                .ok_or_else(|| DeError::NonExsistentID(name.to_owned()))?,
        )
    }

    fn get(&self, id: &str) -> DeResult<&Block> {
        self.blocks
            .get(id)
            .ok_or_else(|| DeError::NonExsistentID(id.to_owned()))
    }
}

fn get_rep(json: &Json) -> Option<&Json> {
    let arr = json.as_array()?;
    match &arr[..] {
        [Json::Number(_), val, ..] => Some(val),
        _ => None,
    }
}
