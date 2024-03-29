use crate::{expr::Expr, sprite::Sprite, statement::Statement};
use ecow::EcoString;
use sb3_stuff::{Index, Value};
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
    sprites: HashMap<EcoString, Sprite>,
    #[serde(skip_deserializing)]
    // FIXME: this should be deserialized from the sprites
    vars: RefCell<HashMap<EcoString, Value>>,
    #[serde(skip_deserializing)]
    // FIXME: this should be deserialized from the sprites
    lists: RefCell<HashMap<EcoString, Vec<Value>>>,
    #[serde(skip_deserializing)]
    proc_args: RefCell<HashMap<EcoString, Vec<Value>>>,
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
    IOError(#[from] std::io::Error),
}

type VMResult<T> = Result<T, VMError>;

impl VM {
    pub fn run(&self) -> VMResult<()> {
        // This should be a `try` block
        let res = (|| {
            for spr in self.sprites.values() {
                for proc in &spr.procs.when_flag_clicked {
                    self.run_proc(spr, proc)?;
                }
            }
            Ok(())
        })();

        match res {
            Err(VMError::StopAll) => Ok(()),
            res => res,
        }
    }

    fn run_proc(&self, sprite: &Sprite, proc: &Statement) -> VMResult<()> {
        match self.run_statement(sprite, proc) {
            Err(VMError::StopThisScript) => Ok(()),
            res => res,
        }
    }

    fn run_statement(&self, sprite: &Sprite, stmt: &Statement) -> VMResult<()> {
        match stmt {
            Statement::Regular { opcode, inputs } => {
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
                for _ in 0..times as u64 {
                    self.run_statement(sprite, body)?;
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
                    .procs
                    .custom
                    .get(proccode)
                    .expect("called non-existent custom procedure");

                match &**proccode {
                    "putchar %s" | "print %s" => {
                        if let Some(s) = args.values().next() {
                            let s = self.eval_expr(sprite, s)?;
                            print!("{s}");
                            std::io::stdout().flush()?;
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

                        self.run_proc(sprite, &proc.body)?;

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
        let mathop = |num: &Expr, f: fn(f64) -> f64| {
            let num = self.eval_expr(sprite, num)?;
            Ok(Value::Num(f(num.to_num())))
        };

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
            Expr::Abs(num) => mathop(num, f64::abs),
            Expr::Floor(num) => mathop(num, f64::floor),
            Expr::Ceiling(num) => mathop(num, f64::ceil),
            Expr::Sqrt(num) => mathop(num, f64::sqrt),
            Expr::Sin(num) => mathop(num, |n| n.to_radians().sin()),
            Expr::Cos(num) => mathop(num, |n| n.to_radians().cos()),
            Expr::Tan(num) => mathop(num, |n| n.to_radians().tan()),
            Expr::Asin(num) => mathop(num, |n| n.to_degrees().asin()),
            Expr::Acos(num) => mathop(num, |n| n.to_degrees().acos()),
            Expr::Atan(num) => mathop(num, |n| n.to_degrees().atan()),
            Expr::Ln(num) => mathop(num, f64::ln),
            Expr::Log(num) => mathop(num, f64::log10),
            Expr::EExp(num) => mathop(num, f64::exp),
            Expr::TenExp(num) => mathop(num, |n| 10.0f64.powf(n)),
            Expr::Call { opcode, inputs } => {
                self.eval_funcall(sprite, opcode, inputs)
            }
        }
    }

    fn input(
        &self,
        sprite: &Sprite,
        inputs: &HashMap<EcoString, Expr>,
        name: &str,
    ) -> VMResult<Value> {
        self.eval_expr(sprite, inputs.get(name).unwrap())
    }

    fn call_builtin_statement(
        &self,
        sprite: &Sprite,
        opcode: &str,
        inputs: &HashMap<EcoString, Expr>,
    ) -> VMResult<()> {
        match opcode {
            "event_broadcastandwait" => {
                let broadcast_input =
                    self.input(sprite, inputs, "BROADCAST_INPUT")?;
                let broadcast_name = broadcast_input.to_cow_str();
                for spr in self.sprites.values() {
                    if let Some(receivers) =
                        spr.procs.broadcasts.get(&*broadcast_name)
                    {
                        for rec in receivers {
                            self.run_proc(sprite, rec)?;
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
            | "pen_setPenSizeTo"
            | "pen_penDown"
            | "pen_penUp"
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
                std::io::stdout().flush()?;
                std::io::stdin().read_line(&mut answer)?;
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
        inputs: &HashMap<EcoString, Expr>,
    ) -> VMResult<Value> {
        let comparison = |ord: cmp::Ordering| {
            let lhs = self.input(sprite, inputs, "OPERAND1")?;
            let rhs = self.input(sprite, inputs, "OPERAND2")?;
            Ok(Value::Bool(lhs.compare(&rhs) == ord))
        };

        let bin_num_op = |f: fn(f64, f64) -> f64| {
            let lhs = self.input(sprite, inputs, "NUM1")?.to_num();
            let rhs = self.input(sprite, inputs, "NUM2")?.to_num();
            Ok(Value::Num(f(lhs, rhs)))
        };

        match opcode {
            "operator_equals" => comparison(cmp::Ordering::Equal),
            "operator_lt" => comparison(cmp::Ordering::Less),
            "operator_gt" => comparison(cmp::Ordering::Greater),
            "operator_not" => {
                let operand = self.input(sprite, inputs, "OPERAND")?.to_bool();
                Ok(Value::Bool(!operand))
            }
            "operator_or" => Ok(Value::Bool(
                self.input(sprite, inputs, "OPERAND1")?.to_bool()
                    || self.input(sprite, inputs, "OPERAND2")?.to_bool(),
            )),
            "operator_and" => Ok(Value::Bool(
                self.input(sprite, inputs, "OPERAND1")?.to_bool()
                    && self.input(sprite, inputs, "OPERAND2")?.to_bool(),
            )),
            "operator_add" => bin_num_op(ops::Add::add),
            "operator_subtract" => bin_num_op(ops::Sub::sub),
            "operator_multiply" => bin_num_op(ops::Mul::mul),
            "operator_divide" => bin_num_op(ops::Div::div),
            "operator_length" => {
                let s =
                    self.eval_expr(sprite, inputs.get("STRING").unwrap())?;
                Ok(Value::Num(s.to_cow_str().len() as f64))
            }
            "operator_join" => {
                let lhs = self.input(sprite, inputs, "STRING1")?;
                let rhs = self.input(sprite, inputs, "STRING2")?;
                Ok(Value::String((lhs.to_cow_str() + rhs.to_cow_str()).into()))
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
                            Index::Nth(i) => Some(Value::String(
                                s.to_cow_str()
                                    .chars()
                                    .skip(i)
                                    .take(1)
                                    .collect(),
                            )),
                            Index::Last => None,
                        }
                    })()
                    .unwrap_or_default(),
                )
            }
            "sensing_answer" => {
                Ok(Value::String(self.answer.borrow().as_str().into()))
            }
            "sensing_timer" => {
                Ok(Value::Num(self.timer.get().elapsed().as_secs_f64()))
            }
            _ => Err(VMError::UnknownOpcode(opcode.to_owned())),
        }
    }
}
