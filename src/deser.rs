use crate::{
    expr::{str_to_num, Expr, Value},
    field::Field,
    proc::{Proc, Signature},
    statement::Statement,
};
use serde::Deserialize;
use serde_json::Value as Json;
use std::{borrow::Cow, collections::HashMap};

pub(crate) struct DeCtx<'a> {
    pub blocks: HashMap<String, Block<'a>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Block<'a> {
    pub opcode: Cow<'a, str>,
    pub parent: Option<String>,
    pub next: Option<String>,
    #[serde(default)]
    pub inputs: HashMap<String, Json>,
    #[serde(default)]
    pub fields: HashMap<String, Json>,
}

impl<'a> DeCtx<'a> {
    // FIXME: This should be able to return an error
    pub fn build_procs(&self) -> Vec<Proc> {
        self.blocks
            .values()
            .filter_map(|block| match &*block.opcode {
                "procedures_definition" => {
                    let next = block.next.as_ref()?;
                    let body = self.build_statement(next);
                    let signature = todo!();
                    Some(Proc { signature, body })
                }
                "event_whenflagclicked" => {
                    let next = block.next.as_ref()?;
                    let body = self.build_statement(next);
                    Some(Proc {
                        signature: Signature::WhenFlagClicked,
                        body,
                    })
                }
                _ => None,
            })
            .collect()
    }

    // FIXME: This should be able to return an error
    fn build_statement(&self, id: &str) -> Statement {
        let block = self.blocks.get(id).unwrap();

        match &*block.opcode {
            "control_if" => {
                let condition = block.inputs.get("CONDITION").unwrap();
                let condition = self.build_expr(condition);
                let if_true = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_true = self.build_statement(if_true);
                Statement::IfElse {
                    condition,
                    if_true: Box::new(if_true),
                    if_false: Box::new(Statement::Do(Vec::new())),
                }
            }
            "control_if_else" => {
                let condition = block.inputs.get("CONDITION").unwrap();
                let condition = self.build_expr(condition);
                let if_true = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_true = self.build_statement(if_true);
                let if_false = block
                    .inputs
                    .get("SUBSTACK2")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let if_false = self.build_statement(if_false);
                Statement::IfElse {
                    condition,
                    if_true: Box::new(if_true),
                    if_false: Box::new(if_false),
                }
            }
            "control_repeat" => todo!(),
            "control_forever" => {
                let body = block
                    .inputs
                    .get("SUBSTACK")
                    .and_then(get_rep)
                    .and_then(Json::as_str)
                    .unwrap();
                let body = self.build_statement(body);
                Statement::Forever {
                    body: Box::new(body),
                }
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
                let body = self.build_statement(body);
                Statement::For {
                    counter: todo!(),
                    times: todo!(),
                    body: Box::new(body),
                }
            }
            opcode => {
                let inputs = block
                    .inputs
                    .iter()
                    .map(|(id, b)| (id.clone(), self.build_expr(b)))
                    .collect();
                let fields = block
                    .fields
                    .iter()
                    .map(|(id, b)| (id.clone(), self.build_field(b)))
                    .collect();
                Statement::Builtin {
                    opcode: opcode.to_string(),
                    inputs,
                    fields,
                }
            }
        }
    }

    // FIXME: This should be able to return an error
    fn build_expr(&self, json: &Json) -> Expr {
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
                    Expr::Lit(Value::Num(num))
                }
                _ => todo!(),
            },
            _ => todo!(),
        }
    }

    // FIXME: This should be able to return an error
    fn build_field(&self, json: &Json) -> Field {
        dbg!(json);
        todo!()
    }

    // FIXME: This should be able to return an error
    fn build_funcall(&self, id: &str) -> Expr {
        let block = self.blocks.get(id).unwrap();
        Expr::Call {
            opcode: block.opcode.to_string(),
            inputs: block
                .inputs
                .iter()
                .map(|(id, inp)| (id.clone(), self.build_expr(inp)))
                .collect(),
            fields: block
                .fields
                .iter()
                .map(|(id, inp)| (id.clone(), self.build_field(inp)))
                .collect(),
        }
    }
}

fn get_rep(json: &Json) -> Option<&Json> {
    let arr = json.as_array()?;
    match &arr[..] {
        [Json::Number(_), val, ..] => Some(val),
        _ => None,
    }
}
