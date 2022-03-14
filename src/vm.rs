use crate::{
    expr::{Expr, Index, Value},
    proc::{Proc, Signature},
    sprite::Sprite,
    statement::Statement,
};
use serde::Deserialize;
use std::{
    cell::{Cell, RefCell},
    cmp,
    collections::HashMap,
    io::Write,
    ops, time,
};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct VM {
    #[serde(rename = "targets")]
    #[serde(deserialize_with = "crate::sprite::deserialize_sprites")]
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
    #[serde(skip_deserializing)]
    #[serde(default = "default_timer")]
    timer: Cell<time::Instant>,
}

fn default_timer() -> Cell<time::Instant> {
    Cell::new(time::Instant::now())
}

#[derive(Debug, Error)]
pub enum VMError {
    #[error("stopped this script")]
    StopThisScript,
    #[error("stopped all scripts")]
    StopAll,
    #[error("unknown opcode: `{0}`")]
    UnknownOpcode(String),
    #[error("IO error: {0}")]
    IOError(std::io::Error),
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
            Statement::If { condition, if_true } => {
                let condition = self.eval_expr(sprite, condition)?.to_bool();
                if condition {
                    self.run_statement(sprite, if_true)
                } else {
                    Ok(())
                }
            }
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
                self.run_statement(sprite, body)?;
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
                        .insert(counter_id.clone(), Value::Num(i as f64));
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::ProcCall { proccode, args } => {
                let proc = sprite
                    .custom_procs
                    .get(proccode)
                    .expect("called non-existent custom procedure");

                match &**proccode {
                    "putchar %s" | "print %s" => {
                        if let Some(s) = args.values().next() {
                            let s = self.eval_expr(sprite, s)?;
                            print!("{s}");
                            std::io::stdout()
                                .flush()
                                .map_err(VMError::IOError)?;
                        }
                    }
                    "println %s" => {
                        if let Some(s) = args.values().next() {
                            let s = self.eval_expr(sprite, s)?;
                            println!("{s}");
                        }
                    }
                    "term-clear" => {
                        println!("\x1b[2J\x1b[H");
                    }
                    _ => {
                        for (id, arg) in args {
                            let arg = self.eval_expr(sprite, arg)?;
                            self.proc_args
                                .borrow_mut()
                                .entry(
                                    proc.arg_names_by_id
                                        .get(id)
                                        .unwrap()
                                        .clone(),
                                )
                                .or_insert_with(|| Vec::with_capacity(1))
                                .push(arg);
                        }

                        match self.run_statement(sprite, &proc.body) {
                            Err(VMError::StopThisScript) => Ok(()),
                            res => res,
                        }?;

                        for id in args.keys() {
                            if let Some(stack) = self
                                .proc_args
                                .borrow_mut()
                                .get_mut(proc.arg_names_by_id.get(id).unwrap())
                            {
                                stack.pop();
                            }
                        }
                    }
                }

                Ok(())
            }
            Statement::DeleteAllOfList { list_id } => {
                // This could be done with a simple `insert` but that would
                // throw away the capacity of the old vector.
                self.lists
                    .borrow_mut()
                    .entry(list_id.clone())
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
                    .entry(list_id.clone())
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
                self.vars.borrow_mut().insert(var_id.clone(), value);
                Ok(())
            }
            Statement::ChangeVariableBy { var_id, value } => {
                let value = self.eval_expr(sprite, value)?.to_num();
                let mut vars = self.vars.borrow_mut();
                let old = vars.get(var_id).map_or(0.0, Value::to_num);
                vars.insert(var_id.clone(), Value::Num(old + value));
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
                    .map_or(0.0, |lst| Vec::len(lst) as f64),
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
        let lhs = self.input(sprite, inputs, "NUM1")?.to_num();
        let rhs = self.input(sprite, inputs, "NUM2")?.to_num();
        Ok(Value::Num(f(lhs, rhs)))
    }

    fn input(
        &self,
        sprite: &Sprite,
        inputs: &HashMap<String, Expr>,
        name: &str,
    ) -> VMResult<Value> {
        self.eval_expr(sprite, inputs.get(name).unwrap())
    }

    fn call_builtin_statement(
        &self,
        sprite: &Sprite,
        opcode: &str,
        inputs: &HashMap<String, Expr>,
    ) -> VMResult<()> {
        match opcode {
            "event_broadcastandwait" => {
                let broadcast_name =
                    self.input(sprite, inputs, "BROADCAST_INPUT")?.to_string();
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
                let x = self.input(sprite, inputs, "X")?.to_num();
                let y = self.input(sprite, inputs, "Y")?.to_num();
                sprite.x.set(x);
                sprite.y.set(y);
                Ok(())
            }
            "motion_setx" => {
                let x = self.input(sprite, inputs, "X")?.to_num();
                sprite.x.set(x);
                Ok(())
            }
            "motion_sety" => {
                let y = self.input(sprite, inputs, "Y")?.to_num();
                sprite.y.set(y);
                Ok(())
            }
            "motion_changexby" => {
                let dx = self.input(sprite, inputs, "DX")?.to_num();
                sprite.x.set(sprite.x.get() + dx);
                Ok(())
            }
            "motion_changeyby" => {
                let dy = self.input(sprite, inputs, "DY")?.to_num();
                sprite.y.set(sprite.y.get() + dy);
                Ok(())
            }
            "pen_clear"
            | "pen_stamp"
            | "looks_show"
            | "looks_hide"
            | "looks_setsizeto"
            | "looks_switchcostumeto" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_say" => {
                let message = self.input(sprite, inputs, "MESSAGE")?;
                println!("{message}");
                Ok(())
            }
            "sensing_askandwait" => {
                let question = self.input(sprite, inputs, "QUESTION")?;
                print!("{question}");
                let mut answer = String::new();
                std::io::stdout().flush().map_err(VMError::IOError)?;
                std::io::stdin()
                    .read_line(&mut answer)
                    .map_err(VMError::IOError)?;
                self.answer.replace(answer.trim().to_owned());
                Ok(())
            }
            "control_wait" => {
                let duration = self.input(sprite, inputs, "DURATION")?;
                std::thread::sleep(time::Duration::from_micros(
                    (duration.to_num() * 1.0e6) as u64,
                ));
                Ok(())
            }
            _ => Err(VMError::UnknownOpcode(opcode.to_owned())),
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
                let lhs = self.input(sprite, inputs, "OPERAND1")?;
                let rhs = self.input(sprite, inputs, "OPERAND2")?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Equal))
            }
            "operator_lt" => {
                let lhs = self.input(sprite, inputs, "OPERAND1")?;
                let rhs = self.input(sprite, inputs, "OPERAND2")?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Less))
            }
            "operator_gt" => {
                let lhs = self.input(sprite, inputs, "OPERAND1")?;
                let rhs = self.input(sprite, inputs, "OPERAND2")?;
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Greater))
            }
            "operator_not" => {
                let operand = self.input(sprite, inputs, "OPERAND")?.to_bool();
                Ok(Value::Bool(!operand))
            }
            "operator_or" => {
                let lhs = self.input(sprite, inputs, "OPERAND1")?.to_bool();
                if lhs {
                    Ok(Value::Bool(true))
                } else {
                    let rhs = self.input(sprite, inputs, "OPERAND2")?.to_bool();
                    Ok(Value::Bool(rhs))
                }
            }
            "operator_and" => {
                let lhs = self.input(sprite, inputs, "OPERAND1")?.to_bool();
                if lhs {
                    let rhs = self.input(sprite, inputs, "OPERAND2")?.to_bool();
                    Ok(Value::Bool(rhs))
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
                Ok(Value::Num(s.to_string().len() as f64))
            }
            "operator_join" => {
                let lhs = self.input(sprite, inputs, "STRING1")?;
                let rhs = self.input(sprite, inputs, "STRING2")?;
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
                let s = self.input(sprite, inputs, "STRING")?;
                let index = self.input(sprite, inputs, "LETTER")?;
                Ok(
                    // This should be a `try` block
                    (|| {
                        let index = index.to_index()?;
                        match index {
                            Index::Nth(i) => Some(Value::Str(
                                s.to_string().chars().nth(i)?.to_string(),
                            )),
                            Index::Last => None,
                        }
                    })()
                    .unwrap_or_default(),
                )
            }
            "sensing_answer" => Ok(Value::Str(self.answer.borrow().clone())),
            "sensing_timer" => Ok(Value::Num(
                self.timer.get().elapsed().as_micros() as f64 * 1.0e-6,
            )),
            _ => Err(VMError::UnknownOpcode(opcode.to_owned())),
        }
    }
}
