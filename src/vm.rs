use crate::{
    expr::{Expr, Value},
    proc::{Proc, Signature},
    sprite::Sprite,
    statement::Statement,
};
use serde::{Deserialize, Deserializer};
use std::{cell::RefCell, cmp, collections::HashMap};

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

type VMResult<T> = Result<T, String>; // TODO: Proper error type

impl VM {
    pub fn run(&self) -> VMResult<()> {
        println!("Running vm...");

        for spr in self.sprites.values() {
            for proc in &spr.procs {
                if let Signature::WhenFlagClicked = proc.signature {
                    self.run_proc(spr, proc)?;
                }
            }
        }

        Ok(())
    }

    fn run_proc(&self, sprite: &Sprite, proc: &Proc) -> VMResult<()> {
        self.run_statement(sprite, &proc.body)
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

                for (id, arg) in args {
                    let arg = self.eval_expr(sprite, arg)?;
                    self.proc_args
                        .borrow_mut()
                        .entry(id.to_string())
                        .or_insert_with(|| Vec::with_capacity(1))
                        .push(arg);
                }

                self.run_proc(sprite, proc)?;

                for id in args.keys() {
                    if let Some(stack) = self.proc_args.borrow_mut().get_mut(id)
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
                if let (Some(lst), Some(index)) =
                    (self.lists.borrow_mut().get_mut(list_id), index.to_index())
                {
                    if index < lst.len() {
                        lst.remove(index);
                    }
                }
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
            Statement::ReplaceItemOfList { .. } => todo!(),
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
            Statement::StopAll => todo!(),
            Statement::StopThisScript => todo!(),
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
                let index = self.eval_expr(sprite, index)?.to_index();
                Ok(match index {
                    Some(index) => self
                        .lists
                        .borrow()
                        .get(list_id)
                        .and_then(|lst| lst.get(index).cloned())
                        .unwrap_or_default(),
                    None => Value::default(),
                })
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

    fn call_builtin_statement(
        &self,
        sprite: &Sprite,
        opcode: &str,
        inputs: &HashMap<String, Expr>,
    ) -> VMResult<()> {
        match opcode {
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
            "looks_show" => {
                // TODO: Actually do something
                Ok(())
            }
            "looks_hide" => {
                // TODO: Actually do something
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
                Ok(Value::Bool(lhs.compare(&rhs) == cmp::Ordering::Greater))
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
            "operator_add" => {
                let lhs =
                    self.eval_expr(sprite, inputs.get("NUM1").unwrap())?;
                let rhs =
                    self.eval_expr(sprite, inputs.get("NUM2").unwrap())?;
                Ok(Value::Num(lhs.to_num() + rhs.to_num()))
            }
            "operator_length" => {
                let s =
                    self.eval_expr(sprite, inputs.get("STRING").unwrap())?;
                // JavaScript uses UTF-16 for some reason
                Ok(Value::Num(s.to_string().encode_utf16().count() as f64))
            }
            "motion_xposition" => {
                // FIXME: This should be rounded
                Ok(Value::Num(sprite.x.get()))
            }
            "motion_yposition" => {
                // FIXME: This should be rounded
                Ok(Value::Num(sprite.y.get()))
            }
            _ => {
                dbg!(opcode);
                dbg!(inputs);
                todo!()
            }
        }
    }
}
