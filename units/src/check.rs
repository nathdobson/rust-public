use std::collections::HashMap;
use ustr::Ustr;
use crate::ast::{Block, Stmt, Expr, Opcode, UnitSet};
use crate::factors::Factors;
use crate::unit::{UnitCtx, UnitError};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum Type {
    NumType(Factors),
}

#[derive(Clone, Debug)]
pub struct TypeCtx {
    types: HashMap<Ustr, Type>,
    unit_ctx: UnitCtx,
}

#[derive(Clone, Debug)]
pub enum TypeError {
    EarlyReturn,
    MissingReturn,
    Redefine,
    Undefined,
    BadOp,
    BadReturn(Factors, Factors),
    UnitsWithUnits,
    AsBadUnit,
    UnitError(UnitError),
}

pub type TypeResult<T> = Result<T, TypeError>;

impl From<UnitError> for TypeError { fn from(x: UnitError) -> Self { TypeError::UnitError(x) } }

impl TypeCtx {
    pub fn new(unit_ctx: UnitCtx) -> Self {
        TypeCtx { types: HashMap::new(), unit_ctx }
    }
    pub fn check(&mut self, block: &Block) -> TypeResult<Type> {
        let mut result = None;
        for stmt in block.0.iter() {
            if result.is_some() {
                return Err(TypeError::EarlyReturn);
            }
            if let Some(t) = self.check_stmt(stmt)? {
                result = Some(t);
            }
        }
        Ok(result.ok_or(TypeError::MissingReturn)?)
    }
    pub fn check_stmt(&mut self, stmt: &Stmt) -> TypeResult<Option<Type>> {
        match stmt {
            Stmt::Let(x, e) => {
                if self.types.insert(*x, self.check_expr(e)?).is_some() {
                    Err(TypeError::Redefine)
                } else {
                    Ok(None)
                }
            }
            Stmt::Return(e) => {
                let t = self.check_expr(e)?;
                // let d1 = self.unit_ctx.unit_data_for_factors(
                //     &self.unit_ctx.factors_for_unit_set(u)
                // )?.dimension;
                // match &t {
                //     Type::NumType(d2) => {
                //         if &d1 != d2 {
                //             return Err(TypeError::BadReturn(d1, d2.clone()));
                //         }
                //     }
                // }
                Ok(Some(t))
            }
        }
    }
    pub fn check_expr(&self, expr: &Expr) -> TypeResult<Type> {
        Ok(match expr {
            Expr::Number(_) =>
                Type::NumType(Factors::new()),
            Expr::Var(x) =>
                self.types.get(x).ok_or(TypeError::Undefined)?.clone(),
            Expr::Op(x, Opcode::Mul, y) =>
                match (self.check_expr(x)?, self.check_expr(y)?) {
                    (Type::NumType(d1), Type::NumType(d2)) =>
                        Type::NumType(&d1 * &d2)
                }
            Expr::Op(x, Opcode::Div, y) =>
                match (self.check_expr(x)?, self.check_expr(y)?) {
                    (Type::NumType(d1), Type::NumType(d2)) =>
                        Type::NumType(&d1 / &d2)
                }
            Expr::Op(x, Opcode::Add | Opcode::Sub, y) =>
                match (self.check_expr(x)?, self.check_expr(y)?) {
                    (Type::NumType(d1), Type::NumType(d2)) => {
                        if d1 == d2 {
                            Type::NumType(d1)
                        } else {
                            return Err(TypeError::BadOp);
                        }
                    }
                }
            Expr::Call(fun, es) => {
                let ts = es.iter()
                    .map(|e| self.check_expr(e))
                    .collect::<TypeResult<Vec<_>>>()?;
                self.check_fun(*fun, ts)?
            }
            Expr::WithUnits(e, u) => {
                match self.check_expr(e)? {
                    Type::NumType(d1) => {
                        if d1 != Factors::new() {
                            return Err(TypeError::UnitsWithUnits);
                        }
                    }
                }
                let conversion = self.unit_ctx.unit_data_for_factors(
                    &self.unit_ctx.factors_for_unit_set(u))?;
                Type::NumType(conversion.dimension)
            }
            Expr::AsUnits(e, u) => {
                let t = self.check_expr(e)?;
                let d1 = match &t { Type::NumType(d1) => d1 };
                for unit_set in u {
                    let conversion = self.unit_ctx.unit_data_for_factors(
                        &self.unit_ctx.factors_for_unit_set(unit_set))?;
                    if &conversion.dimension != d1 {
                        return Err(TypeError::AsBadUnit);
                    }
                }
                t
            }
        })
    }
    pub fn check_fun(&self, fun: Ustr, ts: Vec<Type>) -> TypeResult<Type> {
        todo!()
    }
}

impl Display for TypeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::EarlyReturn => write!(f, "`return' statement before last line"),
            TypeError::MissingReturn => write!(f, "No `return` statement"),
            TypeError::Redefine => write!(f, "Redefining variable"),
            TypeError::Undefined => write!(f, "Undefined variable"),
            TypeError::BadOp => write!(f, "Adding or subtracting different dimensions"),
            TypeError::BadReturn(f1, f2) => write!(f, "Returning `{:?}' as `{:?}'", f2, f1),
            TypeError::UnitsWithUnits => write!(f, "`with units' applied to term already with units"),
            TypeError::UnitError(u) => write!(f, "{}", u),
            TypeError::AsBadUnit => write!(f, "Explicit `as' between dimensions"),
        }
    }
}