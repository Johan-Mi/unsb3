use crate::{
    expr::{str_to_num, Expr, Value},
    field::Field,
    proc::{Proc, Signature},
    statement::Statement,
};
use serde::{Deserialize, Deserializer};
use serde_json::Value as Json;
use std::{borrow::Cow, cell::Cell, collections::HashMap};

#[derive(Debug, Deserialize)]
pub(crate) struct Sprite {
    #[serde(rename = "blocks")]
    #[serde(deserialize_with = "deserialize_blocks")]
    pub procs: Vec<Proc>,
    #[serde(default)]
    pub x: Cell<f64>,
    #[serde(default)]
    pub y: Cell<f64>,
}

#[derive(Debug, Deserialize)]
struct Block<'a> {
    opcode: Cow<'a, str>,
    parent: Option<String>,
    next: Option<String>,
    #[serde(default)]
    inputs: HashMap<String, Json>,
    #[serde(default)]
    fields: HashMap<String, Json>,
}

fn deserialize_blocks<'de, D>(deserializer: D) -> Result<Vec<Proc>, D::Error>
where
    D: Deserializer<'de>,
{
    let blocks = <HashMap<String, Block>>::deserialize(deserializer)?;
    Ok(build_procs(&blocks))
}

// FIXME: This should be able to return an error
fn build_procs(blocks: &HashMap<String, Block>) -> Vec<Proc> {
    blocks
        .values()
        .filter_map(|block| match &*block.opcode {
            "procedures_definition" => todo!(),
            "event_whenflagclicked" => {
                let next = block.next.as_ref()?;
                let body = build_statement(blocks, next);
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
fn build_statement(blocks: &HashMap<String, Block>, id: &str) -> Statement {
    let block = blocks.get(id).unwrap();

    match &*block.opcode {
        "control_if" => todo!(),
        "control_repeat" => todo!(),
        "control_forever" => {
            let body = block
                .inputs
                .get("SUBSTACK")
                .and_then(get_rep)
                .and_then(Json::as_str)
                .unwrap();
            let body = build_statement(blocks, body);
            Statement::Forever {
                body: Box::new(body),
            }
        }
        "control_repeat_until" => todo!(),
        "control_while" => todo!(),
        "control_for_each" => todo!(),
        opcode => {
            let inputs = block
                .inputs
                .iter()
                .map(|(id, b)| (id.clone(), build_expr(blocks, b)))
                .collect();
            let fields = block
                .fields
                .iter()
                .map(|(id, b)| (id.clone(), build_field(blocks, b)))
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
fn build_expr(blocks: &HashMap<String, Block>, json: &Json) -> Expr {
    let rep = get_rep(json).unwrap();
    match rep {
        Json::String(id) => build_funcall(blocks, id),
        Json::Array(arr) => match &arr[..] {
            [Json::Number(n), num] if n == &10.into() => {
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
fn build_field(blocks: &HashMap<String, Block>, json: &Json) -> Field {
    todo!()
}

// FIXME: This should be able to return an error
fn build_funcall(blocks: &HashMap<String, Block>, id: &str) -> Expr {
    todo!()
}

fn get_rep(json: &Json) -> Option<&Json> {
    let arr = json.as_array()?;
    match &arr[..] {
        [Json::Number(_), val, ..] => Some(val),
        _ => None,
    }
}
