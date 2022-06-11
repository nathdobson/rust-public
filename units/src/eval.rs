use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use ordered_float::OrderedFloat;
//use crate::value::Value;
use ustr::{ustr, Ustr};

use crate::ast::{Block, Expr, Opcode, Stmt, UnitPart, UnitSet};
use crate::factors::Factors;
//use crate::value::Value;
use crate::unit::{UnitCtx, UnitData};
use crate::value::Value;
use crate::variants::{Note, VariantFn};

#[derive(Clone)]
pub struct EvalCtx {
    variables: im::HashMap<Ustr, Value>,
    unit_ctx: UnitCtx,
}

impl EvalCtx {
    pub fn new(unit_ctx: UnitCtx) -> Self {
        EvalCtx {
            variables: im::HashMap::new(),
            unit_ctx,
        }
    }
    pub fn eval<'eval>(&self, block: &'eval Block) -> VariantFn<'eval, Value> {
        self.eval_stmts(&block.0)
    }
    fn eval_stmts<'eval>(&self, stmts: &'eval [Stmt]) -> VariantFn<'eval, Value> {
        assert!(!stmts.is_empty());
        match &stmts[0] {
            Stmt::Let(x, e) => {
                let this = self.clone();
                self.eval_expr(&e, None).then(move |v| {
                    let mut new_ctx = this.clone();
                    new_ctx.variables.insert(x.clone(), v);
                    new_ctx.eval_stmts(&stmts[1..])
                })
            }
            Stmt::Return(e) => {
                assert!(stmts.len() == 1);
                self.eval_expr(&e, None)
            }
        }
    }

    fn eval_expr<'eval>(
        &self,
        expr: &'eval Expr,
        target: Option<Factors>,
    ) -> VariantFn<'eval, Value> {
        match expr {
            Expr::Number(n) => VariantFn::correct(Value::number(*n)),
            Expr::Var(x) => VariantFn::correct(self.variables.get(x).unwrap().clone()),
            Expr::Op(e1, op, e2) => {
                let new_ctx = self.clone();
                let child_target = match op {
                    Opcode::Add | Opcode::Sub => target,
                    Opcode::Mul | Opcode::Div => None,
                };
                self.eval_expr(e1, child_target.clone()).then(move |v1| {
                    let new_ctx = new_ctx.clone();
                    new_ctx
                        .eval_expr(e2, child_target.clone())
                        .then(move |v2| new_ctx.eval_op(*op, &v1, &v2, child_target))
                })
            }
            Expr::Call(f, es) => {
                let mut params: VariantFn<'eval, Vec<Value>> = VariantFn::correct(vec![]);
                for e in es {
                    let new_ctx = self.clone();
                    params = params.then(move |vs| {
                        let new_ctx = new_ctx.clone();
                        new_ctx.eval_expr(e, Some(Factors::new())).then(move |v| {
                            let mut vs2 = vs.clone();
                            vs2.push(v.clone());
                            VariantFn::correct(vs2)
                        })
                    });
                }
                let new_ctx = self.clone();
                params.then(move |vs| new_ctx.eval_fun(*f, &vs))
            }
            Expr::WithUnits(e, units) => {
                let units = self.unit_ctx.factors_for_unit_set(units);
                self.eval_expr(e, Some(Factors::new())).then(move |v| {
                    assert_eq!(v.units(), &Factors::new());
                    VariantFn::correct(Value::with_unit(v.value(), units.clone()))
                })
            }
            Expr::AsUnits(e, u) => {
                let this = self.clone();
                let u: Vec<Factors> = u
                    .iter()
                    .map(|u| this.unit_ctx.factors_for_unit_set(u))
                    .collect();
                VariantFn::all_correct(u).then(move |u| {
                    let this = this.clone();
                    this.eval_expr(e, Some(u.clone()))
                        .then(move |v| this.convert(&v, &u))
                })
            }
        }
    }

    fn eval_op(
        &self,
        op: Opcode,
        v1: &Value,
        v2: &Value,
        target: Option<Factors>,
    ) -> VariantFn<'static, Value> {
        match op {
            Opcode::Mul => self.mul(v1, v2, target),
            Opcode::Div => self.div(v1, v2, target),
            Opcode::Add => self.add(v1, v2, target),
            Opcode::Sub => self.sub(v1, v2, target),
        }
    }

    fn eval_fun(&self, fun: Ustr, vs: &[Value]) -> VariantFn<'static, Value> {
        match fun.as_str() {
            "mistake" => todo!(),
            fun => panic!("Unknown function {}", fun),
        }
    }

    pub fn add<'a>(
        &'a self,
        v1: &'a Value,
        v2: &'a Value,
        target: Option<Factors>,
    ) -> VariantFn<'static, Value> {
        if v1.units() == v2.units() {
            VariantFn::correct(Value::with_unit(
                v1.value() + v2.value(),
                v1.units().clone(),
            ))
        } else {
            let v1c = v1.clone();
            let v2c = v2.clone();
            let as_v1 = self.convert(v2, v1.units()).then(move |v2| {
                VariantFn::correct(Value::with_unit(
                    v1c.value() + v2.value(),
                    v1c.units().clone(),
                ))
            });
            let as_v2 = self.convert(v1, v2.units()).then(move |v1| {
                VariantFn::correct(Value::with_unit(
                    v1.value() + v2c.value(),
                    v2c.units().clone(),
                ))
            });
            as_v1.union(as_v2)
        }
    }

    pub fn sub(&self, v1: &Value, v2: &Value) -> VariantFn<'static, Value> {
        assert_eq!(v1.units(), v2.units());
        VariantFn::correct(Value::with_unit(
            v1.value() - v2.value(),
            v1.units().clone(),
        ))
    }

    pub fn mul(&self, v1: &Value, v2: &Value) -> VariantFn<'static, Value> {
        VariantFn::correct(Value::with_unit(
            v1.value() * v2.value(),
            v1.units() * v2.units(),
        ))
    }

    pub fn div(&self, v1: &Value, v2: &Value) -> VariantFn<'static, Value> {
        VariantFn::correct(Value::with_unit(
            v1.value() / v2.value(),
            v1.units() / v2.units(),
        ))
    }
    fn convert(&self, v: &Value, unit: &Factors) -> VariantFn<'static, Value> {
        let canon = self
            .unit_ctx
            .unit_data_for_factors(v.units())
            .unwrap()
            .conversion
            .to_canonical(v.value());
        let conv = self
            .unit_ctx
            .unit_data_for_factors(unit)
            .unwrap()
            .conversion
            .from_canonical(canon);
        VariantFn::correct(Value::with_unit(conv, unit.clone())).union(VariantFn::incorrect(
            Value::with_unit(v.value(), unit.clone()),
            Note::new(ustr(&format!(
                "Skipped unit conversion from `{:?}' to `{:?}'.",
                v.units(),
                unit
            ))),
        ))
    }
}
