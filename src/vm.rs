use crate::{
    expr::{Expr, Value},
    proc::Proc,
    sprite::Sprite,
    statement::Statement,
};
use serde::{Deserialize, Deserializer};
use std::{cell::RefCell, collections::HashMap};

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

        // FIXME: This just runs all scripts in all sprites with no concept of
        // events.
        for spr in self.sprites.values() {
            for proc in &spr.procs {
                self.run_proc(spr, proc)?;
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
                let condition = self.eval_expr(condition)?.to_bool();
                self.run_statement(
                    sprite,
                    if condition { if_true } else { if_false },
                )
            }
            Statement::Repeat { times, body } => {
                let times = self.eval_expr(times)?.to_num().round();
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
                while !self.eval_expr(condition)?.to_bool() {
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                while self.eval_expr(condition)?.to_bool() {
                    self.run_statement(sprite, body)?;
                }
                Ok(())
            }
            Statement::For {
                counter,
                times,
                body,
            } => {
                // FIXME: This does not set the loop variable
                let times = self.eval_expr(times)?.to_num().ceil();
                if times > 0.0 {
                    if times.is_infinite() {
                        for i in 1.. {
                            self.run_statement(sprite, body)?;
                        }
                    } else {
                        for i in 1..=times as u64 {
                            self.run_statement(sprite, body)?;
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
                    .entry(list_id.to_owned())
                    .and_modify(Vec::clear)
                    .or_insert_with(Vec::new);
                Ok(())
            }
            Statement::AddToList { .. } => todo!(),
            Statement::ReplaceItemOfList { .. } => todo!(),
            Statement::SetVariable { var_id, value } => {
                let value = self.eval_expr(value)?;
                self.vars.borrow_mut().insert(var_id.to_owned(), value);
                Ok(())
            }
        }
    }

    pub(crate) fn eval_expr(&self, expr: &Expr) -> VMResult<Value> {
        match expr {
            Expr::Lit(lit) => Ok(lit.clone()),
            Expr::Sym(_) => todo!(),
            Expr::GetVar { .. } => todo!(),
            Expr::ProcArgStringNumber { .. } => todo!(),
            Expr::ItemOfList { .. } => todo!(),
            Expr::Call { opcode, inputs } => todo!(),
        }
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
                let x = self.eval_expr(x)?;
                sprite.x.set(x.to_num());
                Ok(())
            }
            "motion_sety" => {
                let y = inputs.get("Y").unwrap();
                let y = self.eval_expr(y)?;
                sprite.y.set(y.to_num());
                Ok(())
            }
            "motion_changexby" => {
                let dx = inputs.get("DX").unwrap();
                let dx = self.eval_expr(dx)?;
                sprite.x.set(sprite.x.get() + dx.to_num());
                Ok(())
            }
            "motion_changeyby" => {
                let dy = inputs.get("DY").unwrap();
                let dy = self.eval_expr(dy)?;
                sprite.y.set(sprite.y.get() + dy.to_num());
                Ok(())
            }
            _ => {
                dbg!(opcode);
                dbg!(inputs);
                todo!()
            }
        }
    }
}
