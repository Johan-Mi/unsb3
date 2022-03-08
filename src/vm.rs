use crate::{
    expr::{Expr, Index, Value},
    proc::{Proc, Signature},
    sprite::Sprite,
    statement::Statement,
};
use serde::{Deserialize, Deserializer};
use std::{cell::RefCell, cmp, collections::HashMap, io::Write, ops};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub(crate) struct VM {
    #[serde(rename = "targets")]
    #[serde(deserialize_with = "deserialize_sprites")]
    sprites: HashMap<String, Sprite>,
    #[serde(skip_deserializing)]
    // FIXME: this should be deserialized from the sprites
    vars: RefCell<HashMap<String, Value>>,
    #[serde(skip_deserializing)]
    // FIXME: this should be deserialized from the sprites
    lists: RefCell<HashMap<String, Vec<Value>>>,
    #[serde(skip_deserializing)]
    proc_args: RefCell<HashMap<String, Vec<Value>>>,
    #[serde(skip_deserializing)]
    answer: RefCell<String>,
}

fn deserialize_sprites<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Sprite>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct NamedSprite {
        name: String,
        #[serde(flatten)]
        inner: Sprite,
    }

    let sprites = <Vec<NamedSprite>>::deserialize(deserializer)?;

    Ok(sprites
        .into_iter()
        .map(|NamedSprite { name, inner }| (name, inner))
        .collect())
}

#[derive(Debug, Error)]
pub(crate) enum VMError {
    #[error("stopped this script")]
    StopThisScript,
    #[error("stopped all scripts")]
    StopAll,
}

type VMResult<T> = Result<T, VMError>;

impl VM {
    pub fn run(&self) -> VMResult<()> {
        println!("Running vm...");

        // This should be a `try` block
        let res = (|| {
            for spr in self.sprites.values() {
                for proc in &spr.procs {
                    if let Signature::WhenFlagClicked = proc.signature {
                        self.run_proc(spr, proc)?;
                    }
                }
            }
            Ok(())
        })();

        match res {
            Err(VMError::StopAll) => Ok(()),
            res => res,
        }
    }

    fn run_proc(&self, sprite: &Sprite, proc: &Proc) -> VMResult<()> {
        if proc.name_is("putchar %s") {
            let proc_args = self.proc_args.borrow();
            if let Some(chr) =
                proc_args.get("char").and_then(|stack| stack.last())
            {
                print!("{chr}");
                std::io::stdout().flush().unwrap();
            }
        }
        match self.run_statement(sprite, &proc.body) {
            Err(VMError::StopThisScript) => Ok(()),
            res => res,
        }
    }

    fn run_statement(&self, sprite: &Sprite, stmt: &Statement) -> VMResult<()> {
        match stmt {
            Statement::Builtin { opcode, inputs } => {
                self.call_builtin_statement(sprite, opcode, inputs)
            }
            Statement::Do(stmts) => stmts
                .iter()
                .try_for_each(|stmt| self.run_statement(sprite, stmt)),
            Statement::IfElse {
                condition,
                if_true,
                if_false,
            } => {
                let condition = self.eval_expr(sprite, condition)?.to_bool();
                self.run_statement(
                    sprite,
                    if condition { if_true } else { if_false },
                )
            }
            Statement::Repeat { times, body } => {
                let times = self.eval_expr(sprite, times)?.to_num().round();
                if times > 0.0 {
                    if times.is_infinite() {
                        loop {
                            self.run_statement(sprite, body)?;
                        }
                    } else {
                        for _ in 0..times as u64 {
                            self.run_statement(sprite, body)?;
                        }
                    }
                }
                Ok(())
            }
            Statement::Forever { body } => loop {
                self.run_statement(sprite, body)?
            },
            Statement::Until { condition, body } => {
                while !self.eval_expr(sprite, condition)?.to_bool() {
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                while self.eval_expr(sprite, condition)?.to_bool() {
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::For {
                counter_id,
                times,
                body,
            } => {
                let times = self.eval_expr(sprite, times)?.to_num().ceil();
                for i in 1..=times as u64 {
                    self.vars
                        .borrow_mut()
                        .insert(counter_id.to_owned(), Value::Num(i as f64));
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::ProcCall { proccode, args } => {
                let proc =
                    sprite.procs.iter().find(|p| p.name_is(proccode)).unwrap();
                let arg_names_by_id = match &proc.signature {
                    Signature::Custom {
                        arg_names_by_id, ..
                    } => arg_names_by_id,
                    _ => todo!(),
                };

                for (id, arg) in args {
                    let arg = self.eval_expr(sprite, arg)?;
                    self.proc_args
                        .borrow_mut()
                        .entry(arg_names_by_id.get(id).unwrap().clone())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(arg);
                }

                self.run_proc(sprite, proc)?;

                for id in args.keys() {
                    if let Some(stack) = self
                        .proc_args
                        .borrow_mut()
                        .get_mut(arg_names_by_id.get(id).unwrap())
                    {
                        stack.pop();
                    }
                }

                Ok(())
            }
            Statement::DeleteAllOfList { list_id } => {
                // This could be done with a simple `insert` but that would
                // throw away the capacity of the old vector.
                self.lists
                    .borrow_mut()
                    .entry(list_id.to_owned())
                    .and_modify(Vec::clear)
                    .or_insert_with(Vec::new);
                Ok(())
            }
            Statement::DeleteOfList { list_id, index } => {
                let index = self.eval_expr(sprite, index)?;
                // This should be a `try` block
                (|| {
                    let mut lists = self.lists.borrow_mut();
                    let lst = lists.get_mut(list_id)?;
                    let index = index.to_index()?;
                    match index {
                        Index::Nth(i) => {
                            if i < lst.len() {
                                lst.remove(i);
                            }
                        }
                        Index::Last => {
                            lst.pop();
                        }
                    }
                    Some(())
                })();
                Ok(())
            }
            Statement::AddToList { list_id, item } => {
                let item = self.eval_expr(sprite, item)?;
                self.lists
                    .borrow_mut()
                    .entry(list_id.to_owned())
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(item);
                Ok(())
            }
            Statement::ReplaceItemOfList {
                list_id,
                index,
                item,
            } => {
                let index = self.eval_expr(sprite, index)?;
                let item = self.eval_expr(sprite, item)?;
                let mut lists = self.lists.borrow_mut();
                // This should be a `try` block
                (|| {
                    let lst = lists.get_mut(list_id)?;
                    let index = index.to_index()?;
                    let slot = match index {
                        Index::Nth(i) => lst.get_mut(i),
                        Index::Last => lst.last_mut(),
                    }?;
                    *slot = item;
                    Some(())
                })();
                Ok(())
            }
            Statement::SetVariable { var_id, value } => {
                let value = self.eval_expr(sprite, value)?;
                self.vars.borrow_mut().insert(var_id.to_owned(), value);
                Ok(())
            }
            Statement::ChangeVariableBy { var_id, value } => {
                let value = self.eval_expr(sprite, value)?.to_num();
                let mut vars = self.vars.borrow_mut();
                let old = vars.get(var_id).map(Value::to_num).unwrap_or(0.0);
                vars.insert(var_id.to_owned(), Value::Num(old + value));
                Ok(())
            }
            Statement::StopAll => Err(VMError::StopAll),
            Statement::StopThisScript => Err(VMError::StopThisScript),
        }
    }

    pub(crate) fn eval_expr(
        &self,
        sprite: &Sprite,
        expr: &Expr,
    ) -> VMResult<Value> {
        match expr {
            Expr::Lit(lit) => Ok(lit.clone()),
            Expr::GetVar { var_id } => {
                Ok(self.vars.borrow().get(var_id).cloned().unwrap_or_default())
            }
            Expr::ProcArgStringNumber { name } => Ok(self
                .proc_args
                .borrow()
                .get(name)
                .and_then(|stack| stack.last().cloned())
                .unwrap_or_default()),
            Expr::ItemOfList { list_id, index } => {
                let index = self.eval_expr(sprite, index)?;
                // This should be a `try` block
                Ok((|| {
                    let lists = self.lists.borrow();
                    let lst = lists.get(list_id)?;
                    let index = index.to_index()?;
                    match index {
                        Index::Nth(i) => lst.get(i),
                        Index::Last => lst.last(),
                    }
                    .cloned()
                })()
                .unwrap_or_default())
            }
            Expr::LengthOfList { list_id } => Ok(Value::Num(
                self.lists
                    .borrow()
                    .get(list_id)
                    .map(|lst| Vec::len(lst) as f64)
                    .unwrap_or(0.0),
            )),
            Expr::Abs(num) => self.mathop(sprite, num, f64::abs),
            Expr::Floor(num) => self.mathop(sprite, num, f64::floor),
            Expr::Ceiling(num) => self.mathop(sprite, num, f64::ceil),
            Expr::Sqrt(num) => self.mathop(sprite, num, f64::sqrt),
            Expr::Sin(num) => self.mathop(sprite, num, f64::sin),
            Expr::Cos(num) => self.mathop(sprite, num, f64::cos),
            Expr::Tan(num) => self.mathop(sprite, num, f64::tan),
            Expr::Asin(num) => self.mathop(sprite, num, f64::asin),
            Expr::Acos(num) => self.mathop(sprite, num, f64::acos),
            Expr::Atan(num) => self.mathop(sprite, num, f64::atan),
            Expr::Ln(num) => self.mathop(sprite, num, f64::ln),
            Expr::Log(num) => self.mathop(sprite, num, f64::log10),
            Expr::EExp(num) => self.mathop(sprite, num, f64::exp),
            Expr::TenExp(num) => self.mathop(sprite, num, |n| 10.0f64.powf(n)),
            Expr::Call { opcode, inputs } => {
                self.eval_funcall(sprite, opcode, inputs)
            }
        }
    }

    fn mathop(
        &self,
        sprite: &Sprite,
        num: &Expr,
        f: fn(f64) -> f64,
    ) -> VMResult<Value> {
        let num = self.eval_expr(sprite, num)?;
        Ok(Value::Num(f(num.to_num())))
    }

    fn bin_num_op(
        &self,
        sprite: &Sprite,
        inputs: &HashMap<String, Expr>,
        f: fn(f64, f64) -> f64,
    ) -> VMResult<Value> {
        let lhs = self.eval_expr(sprite, inputs.get("NUM1").unwrap())?;
        let rhs = self.eval_expr(sprite, inputs.get("NUM2").unwrap())?;
        Ok(Value::Num(f(lhs.to_num(), rhs.to_num())))
    }

    fn call_builtin_statement(
        &self,
        sprite: &Sprite,
        opcode: &str,
        inputs: &HashMap<String, Expr>,
    ) -> VMResult<()> {
        match opcode {
            "event_broadcastandwait" => {
                let broadcast_name = inputs.get("BROADCAST_INPUT").unwrap();
                let broadcast_name =
                    self.eval_expr(sprite, broadcast_name)?.to_string();
                for spr in self.sprites.values() {
                    for proc in &spr.procs {
                        if proc.is_the_broadcast(&broadcast_name) {
                            self.run_proc(spr, proc)?;
                        }
                    }
                }
                Ok(())
            }
            "motion_gotoxy" => {
                let x = inputs.get("X").unwrap();
                let x = self.eval_expr(sprite, x)?;
                let y = inputs.get("Y").unwrap();
                let y = self.eval_expr(sprite, y)?;
                sprite.x.set(x.to_num());
                sprite.y.set(y.to_num());
                Ok(())
            }
            "motion_setx" => {
                let x = inputs.get("X").unwrap();
                let x = self.eval_expr(sprite, x)?;
                sprite.x.set(x.to_num());
                Ok(())
            }
            "motion_sety" => {
                let y = inputs.get("Y").unwrap();
                let y = self.eval_expr(sprite, y)?;
                sprite.y.set(y.to_num());
                Ok(())
            }
            "motion_changexby" => {
                let dx = inputs.get("DX").unwrap();
                let dx = self.eval_expr(sprite, dx)?;
                sprite.x.set(sprite.x.get() + dx.to_num());
                Ok(())
            }
            "motion_changeyby" => {
                let dy = inputs.get("DY").unwrap();
                let dy = self.eval_expr(sprite, dy)?;
                sprite.y.set(sprite.y.get() + dy.to_num());
                Ok(())
            }
            "pen_clear" => {
                // TODO: Actually do something
                Ok(())
            }
            "pen_stamp" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_show" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_hide" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_setsizeto" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_switchcostumeto" => {
                // TODO: Actually do something
                Ok(())
            }
            "sensing_askandwait" => {
                let question = inputs.get("QUESTION").unwrap();
                let question = self.eval_expr(sprite, question)?;
                print!("{question}");
                let mut answer = String::new();
                std::io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut answer).unwrap();
                self.answer.replace(answer.trim().to_owned());
                Ok(())
            }
            _ => {
                dbg!(opcode);
                dbg!(inputs);
                todo!()
            }
        }
    }

    fn eval_funcall(
        &self,
        sprite: &Sprite,
        opcode: &str,
        inputs: &HashMap<String, Expr>,
    ) -> VMResult<Value> {
        match opcode {
            "operator_equals" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("OPERAND1").unwrap())?;
                let rhs =
                    self.eval_expr(sprite, inputs.get("OPERAND2").unwrap())?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Equal))
            }
            "operator_lt" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("OPERAND1").unwrap())?;
                let rhs =
                    self.eval_expr(sprite, inputs.get("OPERAND2").unwrap())?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Less))
            }
            "operator_gt" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("OPERAND1").unwrap())?;
                let rhs =
                    self.eval_expr(sprite, inputs.get("OPERAND2").unwrap())?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Greater))
            }
            "operator_not" => {
                let operand =
                    self.eval_expr(sprite, inputs.get("OPERAND").unwrap())?;
                Ok(Value::Bool(!operand.to_bool()))
            }
            "operator_or" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("OPERAND1").unwrap())?;
                if lhs.to_bool() {
                    Ok(Value::Bool(true))
                } else {
                    let rhs = self
                        .eval_expr(sprite, inputs.get("OPERAND2").unwrap())?;
                    Ok(Value::Bool(rhs.to_bool()))
                }
            }
            "operator_and" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("OPERAND1").unwrap())?;
                if lhs.to_bool() {
                    let rhs = self
                        .eval_expr(sprite, inputs.get("OPERAND2").unwrap())?;
                    Ok(Value::Bool(rhs.to_bool()))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            "operator_add" => self.bin_num_op(sprite, inputs, ops::Add::add),
            "operator_subtract" => {
                self.bin_num_op(sprite, inputs, ops::Sub::sub)
            }
            "operator_multiply" => {
                self.bin_num_op(sprite, inputs, ops::Mul::mul)
            }
            "operator_divide" => self.bin_num_op(sprite, inputs, ops::Div::div),
            "operator_length" => {
                let s =
                    self.eval_expr(sprite, inputs.get("STRING").unwrap())?;
                // JavaScript uses UTF-16 for some reason
                Ok(Value::Num(s.to_string().encode_utf16().count() as f64))
            }
            "operator_join" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("STRING1").unwrap())?;
                let rhs =
                    self.eval_expr(sprite, inputs.get("STRING2").unwrap())?;
                Ok(Value::Str(format!("{lhs}{rhs}")))
            }
            "motion_xposition" => {
                // FIXME: This should be rounded
                Ok(Value::Num(sprite.x.get()))
            }
            "motion_yposition" => {
                // FIXME: This should be rounded
                Ok(Value::Num(sprite.y.get()))
            }
            "operator_letter_of" => {
                let s =
                    self.eval_expr(sprite, inputs.get("STRING").unwrap())?;
                let index =
                    self.eval_expr(sprite, inputs.get("LETTER").unwrap())?;
                // FIXME: This should use UTF-16
                Ok(
                    // This should be a `try` block
                    (|| {
                        let index = index.to_index()?;
                        match index {
                            Index::Nth(i) => Some(Value::Str(
                                s.to_string().get(i..=i)?.to_owned(),
                            )),
                            _ => None,
                        }
                    })()
                    .unwrap_or_default(),
                )
            }
            "sensing_answer" => Ok(Value::Str(self.answer.borrow().clone())),
            _ => {
                dbg!(opcode);
                dbg!(inputs);
                todo!()
            }
        }
    }
}
