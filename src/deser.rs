use crate::{
    expr::Expr,
    proc::{Custom, Procs},
    statement::Statement,
};
use ecow::EcoString;
use sb3_stuff::Value;
use serde::Deserialize;
use serde_json::Value as Json;
use std::{borrow::Cow, collections::HashMap, fmt::Display};
use thiserror::Error;

pub struct DeCtx<'a> {
    blocks: HashMap<EcoString, Block<'a>>,
}

#[derive(Debug, Error)]
pub enum DeError {
    #[error("{0}")]
    Custom(String),
    #[error("found no block with ID `{0}`")]
    NonExsistentID(String),
    #[error("missing input `{0}`")]
    MissingInput(String),
    #[error("missing mutation for block that requires it")]
    MissingMutation,
}

type DeResult<T> = Result<T, DeError>;

impl serde::de::Error for DeError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct Block<'a> {
    #[serde(borrow)]
    pub opcode: Cow<'a, str>,
    // pub parent: Option<String>,
    #[serde(borrow)]
    pub next: Option<Cow<'a, str>>,
    #[serde(default)]
    #[serde(borrow)]
    pub inputs: HashMap<Cow<'a, str>, Json>,
    #[serde(default)]
    #[serde(borrow)]
    pub fields: HashMap<Cow<'a, str>, Json>,
    pub mutation: Option<Mutation<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct Mutation<'a> {
    #[serde(borrow)]
    proccode: Option<Cow<'a, str>>,
    argumentids: Option<String>,
    argumentnames: Option<String>,
}

impl<'a> DeCtx<'a> {
    pub const fn new(blocks: HashMap<EcoString, Block<'a>>) -> Self {
        Self { blocks }
    }

    pub fn build_procs(&self) -> DeResult<Procs> {
        let mut when_flag_clicked = Vec::new();
        let mut custom = HashMap::new();
        let mut broadcasts = HashMap::new();

        for block in self.blocks.values() {
            match &*block.opcode {
                "procedures_definition" => {
                    if let Some(next) = block.next.as_ref() {
                        let body = self.build_statement(next)?;
                        let proto_id = block
                            .inputs
                            .get("custom_block")
                            .and_then(get_rep)
                            .and_then(Json::as_str)
                            .expect("missing prototype for custom block");
                        let proto = self.get(proto_id)?;
                        let mutation = proto
                            .mutation
                            .as_ref()
                            .ok_or(DeError::MissingMutation)?;
                        let name = mutation
                            .proccode
                            .as_ref()
                            .expect("missing proccode for custom block")
                            .to_string();
                        let arg_ids: Vec<EcoString> = serde_json::from_str(
                            mutation
                                .argumentids
                                .as_deref()
                                .expect("missing argumentids"),
                        )
                        .expect("argumentids was not valid JSON");
                        let arg_names: Vec<EcoString> = serde_json::from_str(
                            mutation
                                .argumentnames
                                .as_ref()
                                .expect("missing argumentnames"),
                        )
                        .expect("argumentnames was not valid JSON");
                        let arg_names_by_id = arg_ids
                            .into_iter()
                            .zip(arg_names.into_iter())
                            .collect();
                        custom.insert(
                            name,
                            Custom {
                                arg_names_by_id,
                                body,
                            },
                        );
                    }
                }
                "event_whenflagclicked" => {
                    if let Some(next) = block.next.as_ref() {
                        let body = self.build_statement(next)?;
                        when_flag_clicked.push(body);
                    }
                }
                "event_whenbroadcastreceived" => {
                    if let Some(next) = block.next.as_ref() {
                        let broadcast_name =
                            str_field(block, "BROADCAST_OPTION")?.to_owned();
                        let body = self.build_statement(next)?;
                        broadcasts
                            .entry(broadcast_name)
                            .or_insert_with(|| Vec::with_capacity(1))
                            .push(body);
                    }
                }
                _ => {}
            }
        }

        Ok(Procs {
            when_flag_clicked,
            custom,
            broadcasts,
        })
    }

    fn build_statement(&self, id: &str) -> DeResult<Statement> {
        let block = self.get(id)?;

        if block.next.is_some() {
            let mut blocks = Vec::new();
            let mut pending = Some(block);

            while let Some(curr) = pending {
                blocks.push(self.build_single_statement(curr)?);
                pending = match &curr.next {
                    Some(next) => Some(self.get(next)?),
                    None => None,
                }
            }

            Ok(Statement::Do(blocks))
        } else {
            self.build_single_statement(block)
        }
    }

    fn build_single_statement(&self, block: &Block) -> DeResult<Statement> {
        match &*block.opcode {
            "control_if" => {
                let condition = self.input(block, "CONDITION")?;
                let if_true = self.substack(block, "SUBSTACK")?;
                Ok(Statement::If {
                    condition,
                    if_true: Box::new(if_true),
                })
            }
            "control_if_else" => {
                let condition = self.input(block, "CONDITION")?;
                let if_true = self.substack(block, "SUBSTACK")?;
                let if_false = self.substack(block, "SUBSTACK2")?;
                Ok(Statement::IfElse {
                    condition,
                    if_true: Box::new(if_true),
                    if_false: Box::new(if_false),
                })
            }
            "control_repeat" => {
                let times = self.input(block, "TIMES")?;
                let body = Box::new(self.substack(block, "SUBSTACK")?);
                Ok(Statement::Repeat { times, body })
            }
            "control_forever" => {
                let body = self.substack(block, "SUBSTACK")?;
                Ok(Statement::Forever {
                    body: Box::new(body),
                })
            }
            "control_repeat_until" => {
                let condition = self.input(block, "CONDITION")?;
                let body = Box::new(self.substack(block, "SUBSTACK")?);
                Ok(Statement::Until { condition, body })
            }
            "control_while" => {
                let condition = self.input(block, "CONDITION")?;
                let body = Box::new(self.substack(block, "SUBSTACK")?);
                Ok(Statement::While { condition, body })
            }
            "control_for_each" => {
                let counter_id = var_list_field(block, "VARIABLE")?.into();
                let times = self.input(block, "VALUE")?;
                let body = Box::new(self.substack(block, "SUBSTACK")?);
                Ok(Statement::For {
                    counter_id,
                    times,
                    body,
                })
            }
            "procedures_call" => {
                let mutation =
                    block.mutation.as_ref().ok_or(DeError::MissingMutation)?;
                let proccode = mutation
                    .proccode
                    .as_ref()
                    .expect("missing proccode for custom block")
                    .to_string();
                let args = block
                    .inputs
                    .iter()
                    .map(|(id, arg)| Ok(((**id).into(), self.build_expr(arg)?)))
                    .collect::<Result<_, _>>()?;
                Ok(Statement::ProcCall { proccode, args })
            }
            "data_deletealloflist" => {
                let list_id = var_list_field(block, "LIST")?.into();
                Ok(Statement::DeleteAllOfList { list_id })
            }
            "data_deleteoflist" => {
                let list_id = var_list_field(block, "LIST")?.into();
                let index = self.input(block, "INDEX")?;
                Ok(Statement::DeleteOfList { list_id, index })
            }
            "data_addtolist" => {
                let list_id = var_list_field(block, "LIST")?.into();
                let item = self.input(block, "ITEM")?;
                Ok(Statement::AddToList { list_id, item })
            }
            "data_replaceitemoflist" => {
                let list_id = var_list_field(block, "LIST")?.into();
                let index = self.input(block, "INDEX")?;
                let item = self.input(block, "ITEM")?;
                Ok(Statement::ReplaceItemOfList {
                    list_id,
                    index,
                    item,
                })
            }
            "data_setvariableto" => {
                let var_id = var_list_field(block, "VARIABLE")?.into();
                let value = self.input(block, "VALUE")?;
                Ok(Statement::SetVariable { var_id, value })
            }
            "data_changevariableby" => {
                let var_id = var_list_field(block, "VARIABLE")?.into();
                let value = self.input(block, "VALUE")?;
                Ok(Statement::ChangeVariableBy { var_id, value })
            }
            "control_stop" => {
                let stop_option = str_field(block, "STOP_OPTION")?;
                match stop_option {
                    "all" => Ok(Statement::StopAll),
                    "this script" => Ok(Statement::StopThisScript),
                    _ => {
                        dbg!(stop_option);
                        todo!()
                    }
                }
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
                    .map(|(id, b)| Ok(((**id).into(), self.build_expr(b)?)))
                    .collect::<Result<_, _>>()?;
                Ok(Statement::Regular {
                    opcode: opcode.into(),
                    inputs,
                })
            }
        }
    }

    fn build_expr(&self, json: &Json) -> DeResult<Expr> {
        let rep = get_rep(json).expect("invalid reporter");
        match rep {
            Json::String(id) => self.build_funcall(id),
            Json::Array(arr) => match &arr[..] {
                [Json::Number(n), num]
                    if *n == serde_json::Number::from(4u32) =>
                {
                    let num = match num {
                        Json::String(s) => serde_json::from_str(s)
                            .expect("could not parse number"),
                        _ => todo!(),
                    };
                    Ok(Expr::Lit(Value::Num(num)))
                }
                [Json::Number(n), num]
                    if *n == serde_json::Number::from(5u32) =>
                {
                    let num = match num {
                        Json::String(s) => serde_json::from_str(s)
                            .expect("could not parse positive number"),
                        _ => todo!(),
                    };
                    Ok(Expr::Lit(Value::Num(num)))
                }
                [Json::Number(n), num]
                    if *n == serde_json::Number::from(6u32) =>
                {
                    let num = match num {
                        Json::String(s) => s
                            .parse::<u64>()
                            .expect("could not parse positive integer")
                            as f64,
                        _ => todo!(),
                    };
                    Ok(Expr::Lit(Value::Num(num)))
                }
                [Json::Number(n), s]
                    if *n == serde_json::Number::from(10u32) =>
                {
                    let Json::String(s) = s else {
                        todo!();
                    };
                    Ok(Expr::Lit(Value::String((**s).into())))
                }
                [Json::Number(n), Json::String(_), Json::String(var_id)]
                    if *n == serde_json::Number::from(12u32) =>
                {
                    Ok(Expr::GetVar {
                        var_id: (**var_id).into(),
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
                let name = str_field(block, "VALUE")?.into();
                Ok(Expr::ProcArgStringNumber { name })
            }
            "data_itemoflist" => {
                let index = self.input(block, "INDEX")?;
                let list_id = var_list_field(block, "LIST")?.into();
                Ok(Expr::ItemOfList {
                    list_id,
                    index: Box::new(index),
                })
            }
            "data_lengthoflist" => {
                let list_id = var_list_field(block, "LIST")?.into();
                Ok(Expr::LengthOfList { list_id })
            }
            "operator_mathop" => {
                let operator = str_field(block, "OPERATOR")?;
                let num = self.input(block, "NUM")?;
                match operator {
                    "abs" => Ok(Expr::Abs(Box::new(num))),
                    "floor" => Ok(Expr::Floor(Box::new(num))),
                    "ceiling" => Ok(Expr::Ceiling(Box::new(num))),
                    "sqrt" => Ok(Expr::Sqrt(Box::new(num))),
                    "sin" => Ok(Expr::Sin(Box::new(num))),
                    "cos" => Ok(Expr::Cos(Box::new(num))),
                    "tan" => Ok(Expr::Tan(Box::new(num))),
                    "asin" => Ok(Expr::Asin(Box::new(num))),
                    "acos" => Ok(Expr::Acos(Box::new(num))),
                    "atan" => Ok(Expr::Atan(Box::new(num))),
                    "ln" => Ok(Expr::Ln(Box::new(num))),
                    "log" => Ok(Expr::Log(Box::new(num))),
                    "e ^" => Ok(Expr::EExp(Box::new(num))),
                    "10 ^" => Ok(Expr::TenExp(Box::new(num))),
                    _ => todo!(),
                }
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
                    .map(|(id, inp)| Ok(((**id).into(), self.build_expr(inp)?)))
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
                .ok_or_else(|| DeError::MissingInput(name.to_owned()))?,
        )
    }

    fn get(&self, id: &str) -> DeResult<&Block> {
        self.blocks
            .get(id)
            .ok_or_else(|| DeError::NonExsistentID(id.to_owned()))
    }

    fn substack(&self, block: &Block, name: &str) -> DeResult<Statement> {
        match block.inputs.get(name).and_then(get_rep) {
            Some(Json::String(id)) => self.build_statement(id),
            Some(Json::Null) | None => Ok(Statement::Do(Vec::new())),
            _ => todo!(),
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

fn var_list_field<'blk>(block: &'blk Block, name: &str) -> DeResult<&'blk str> {
    let arr = block
        .fields
        .get(name)
        .and_then(Json::as_array)
        .expect("invalid field");
    match &arr[..] {
        [Json::String(_), Json::String(id)] => Ok(id),
        _ => todo!(),
    }
}

fn str_field<'blk>(block: &'blk Block, name: &str) -> DeResult<&'blk str> {
    let arr = block
        .fields
        .get(name)
        .and_then(Json::as_array)
        .expect("invalid field");
    match &arr[..] {
        [Json::String(s), Json::Null] => Ok(s),
        _ => todo!(),
    }
}
