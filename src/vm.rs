use crate::{
    expr::{Expr, Value},
    proc::Proc,
    sprite::Sprite,
    statement::Statement,
};
use std::collections::HashMap;

pub(crate) struct VM {
    pub sprites: HashMap<String, Sprite>,
}

type VMResult<T> = Result<T, String>; // TODO: Proper error type

impl VM {
    pub fn run(&self) -> VMResult<()> {
        println!("Running vm...");

        // FIXME: This just runs all scripts in all sprites with no concept of
        // events.
        for spr in self.sprites.values() {
            for proc in &spr.procs {
                self.run_proc(proc)?;
            }
        }

        Ok(())
    }

    fn run_proc(&self, proc: &Proc) -> VMResult<()> {
        self.run_statement(&proc.body)
    }

    fn run_statement(&self, stmt: &Statement) -> VMResult<()> {
        match stmt {
            Statement::Call { proc_name, args } => {
                self.call_proc(proc_name, args)
            }
            Statement::Do(stmts) => {
                stmts.iter().try_for_each(|stmt| self.run_statement(stmt))
            }
            Statement::IfElse {
                condition,
                if_true,
                if_false,
            } => {
                let condition = self.eval_expr(condition)?.to_bool();
                self.run_statement(if condition { if_true } else { if_false })
            }
            Statement::Repeat { times, body } => {
                let times = self.eval_expr(times)?.to_num().round();
                if times > 0.0 {
                    if times.is_infinite() {
                        loop {
                            self.run_statement(body)?;
                        }
                    } else {
                        for _ in 0..times as u64 {
                            self.run_statement(body)?;
                        }
                    }
                }
                Ok(())
            }
            Statement::Forever { body } => loop {
                self.run_statement(body)?
            },
            Statement::Until { condition, body } => {
                while !self.eval_expr(condition)?.to_bool() {
                    self.run_statement(body)?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                while self.eval_expr(condition)?.to_bool() {
                    self.run_statement(body)?;
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
                            self.run_statement(body)?;
                        }
                    } else {
                        for i in 1..=times as u64 {
                            self.run_statement(body)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    pub(crate) fn eval_expr(&self, expr: &Expr) -> VMResult<Value> {
        match expr {
            Expr::Lit(lit) => Ok(lit.clone()),
            Expr::Sym(_) => todo!(),
            Expr::Call { func_name, args } => todo!(),
        }
    }

    fn call_proc(&self, proc_name: &str, args: &[Expr]) -> VMResult<()> {
        match proc_name {
            "print" => {
                for arg in args {
                    let arg = self.eval_expr(arg)?;
                    println!("{}", arg.to_string());
                }
                Ok(())
            }

            _ => todo!(),
        }
    }
}
