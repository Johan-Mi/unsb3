use crate::{
    expr::{Expr, Value},
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
    pub next: Option<Cow<'a, str>>,
    #[serde(default)]
    pub inputs: HashMap<Cow<'a, str>, Json>,
    #[serde(default)]
    pub fields: HashMap<Cow<'a, str>, Json>,
    pub mutation: Option<Mutation<'a>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Mutation<'a> {
    proccode: Cow<'a, str>,
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
                        let proto_id = block
                            .inputs
                            .get("custom_block")
                            .and_then(get_rep)
                            .and_then(Json::as_str)
                            .unwrap();
                        let proto = self.get(proto_id)?;
                        let param_ids =
                            proto.inputs.keys().map(Cow::to_string).collect();
                        let mutation = proto.mutation.as_ref().unwrap();
                        let name = mutation.proccode.to_string();
                        let signature = Signature::Custom { name, param_ids };
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
                "event_whenbroadcastreceived" => {
                    let next = block.next.as_ref()?;
                    // This should be a `try` block
                    Some((|| {
                        let broadcast_name =
                            block.fields.get("BROADCAST_OPTION").unwrap();
                        let arr = broadcast_name.as_array().unwrap();
                        let broadcast_name = match &arr[..] {
                            [Json::String(name), Json::Null] => name.to_owned(),
                            _ => todo!(),
                        };
                        let body = self.build_statement(next)?;
                        Ok(Proc {
                            signature: Signature::WhenBroadcastReceived {
                                broadcast_name,
                            },
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
                let counter = var_list_field(block, "VARIABLE")?.to_owned();
                let times = self.input(block, "VALUE")?;
                let body = Box::new(self.substack(block, "SUBSTACK")?);
                Ok(Statement::For {
                    counter,
                    times,
                    body,
                })
            }
            "data_deletealloflist" => {
                let list_id = var_list_field(block, "LIST")?.to_owned();
                Ok(Statement::DeleteAllOfList { list_id })
            }
            "data_addtolist" => {
                let list_id = var_list_field(block, "LIST")?.to_owned();
                let item = self.input(block, "ITEM")?;
                Ok(Statement::AddToList { list_id, item })
            }
            "data_replaceitemoflist" => {
                let list_id = var_list_field(block, "LIST")?.to_owned();
                let index = self.input(block, "INDEX")?;
                let item = self.input(block, "ITEM")?;
                Ok(Statement::ReplaceItemOfList {
                    list_id,
                    index,
                    item,
                })
            }
            "data_setvariableto" => {
                let var_id = var_list_field(block, "VARIABLE")?.to_owned();
                let value = self.input(block, "VALUE")?;
                Ok(Statement::SetVariable { var_id, value })
            }
            opcode => {
                // Field generation has to be done manually for each opcode that uses it
                if !block.fields.is_empty() {
                    dbg!(block);
                    todo!();
                }

                let inputs = block
                    .inputs
                    .iter()
                    .map(|(id, b)| Ok((id.to_string(), self.build_expr(b)?)))
                    .collect::<Result<_, _>>()?;
                Ok(Statement::Builtin {
                    opcode: opcode.to_string(),
                    inputs,
                })
            }
        }
    }

    fn build_expr(&self, json: &Json) -> DeResult<Expr> {
        let rep = get_rep(json).unwrap();
        match rep {
            Json::String(id) => self.build_funcall(id),
            Json::Array(arr) => match &arr[..] {
                [Json::Number(n), s]
                    if n == &serde_json::Number::from(10u32) =>
                {
                    let s = match s {
                        Json::String(s) => s,
                        _ => todo!(),
                    };
                    Ok(Expr::Lit(Value::Str(s.to_owned())))
                }
                [Json::Number(n), Json::String(_), Json::String(var_id)]
                    if n == &serde_json::Number::from(12u32) =>
                {
                    Ok(Expr::GetVar {
                        var_id: var_id.to_owned(),
                    })
                }
                arr => {
                    dbg!(arr);
                    todo!()
                }
            },
            _ => todo!(),
        }
    }

    fn build_funcall(&self, id: &str) -> DeResult<Expr> {
        let block = self.get(id)?;

        match &*block.opcode {
            "argument_reporter_string_number" => {
                let name = block.fields.get("VALUE").unwrap();
                let arr = name.as_array().unwrap();
                let name = match &arr[..] {
                    [Json::String(name), Json::Null] => name.to_owned(),
                    _ => todo!(),
                };
                Ok(Expr::ProcArgStringNumber { name })
            }
            "data_itemoflist" => {
                let index = self.input(block, "INDEX")?;
                let list_id = var_list_field(block, "LIST")?.to_owned();
                Ok(Expr::ItemOfList {
                    list_id,
                    index: Box::new(index),
                })
            }
            opcode => {
                // Field generation has to be done manually for each opcode that uses it
                if !block.fields.is_empty() {
                    dbg!(block);
                    todo!();
                }

                let inputs = block
                    .inputs
                    .iter()
                    .map(|(id, inp)| {
                        Ok((id.to_string(), self.build_expr(inp)?))
                    })
                    .collect::<Result<_, _>>()?;
                Ok(Expr::Call {
                    opcode: opcode.to_string(),
                    inputs,
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

    fn substack(&self, block: &Block, name: &str) -> DeResult<Statement> {
        let stack = block
            .inputs
            .get("SUBSTACK")
            .and_then(get_rep)
            .and_then(Json::as_str)
            .unwrap();
        self.build_statement(stack)
    }
}

fn get_rep(json: &Json) -> Option<&Json> {
    let arr = json.as_array()?;
    match &arr[..] {
        [Json::Number(_), val, ..] => Some(val),
        _ => None,
    }
}

fn var_list_field<'blk>(block: &'blk Block, name: &str) -> DeResult<&'blk str> {
    let arr = block.fields.get(name).and_then(Json::as_array).unwrap();
    match &arr[..] {
        [Json::String(_), Json::String(id)] => Ok(id),
        _ => todo!(),
    }
}
