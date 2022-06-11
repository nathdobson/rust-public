use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::iter;
use std::ops::{Add, Deref, Div, Mul, Sub};

use ordered_float::OrderedFloat;
use ustr::{ustr, Ustr};

use crate::factors::Factors;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Value {
    value: OrderedFloat<f64>,
    units: Factors,
}

#[derive(Clone, Hash, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValueError {}

type ValueResult = Result<Value, ValueError>;

// impl Add for &Value {
//     type Output = Value;
//     fn add(self, rhs: Self) -> Self::Output { Value(self.0 + rhs.0) }
// }
//
// impl Sub for &Value {
//     type Output = Value;
//     fn sub(self, rhs: Self) -> Self::Output { Value(self.0 - rhs.0) }
// }
//
// impl Mul for &Value {
//     type Output = Value;
//     fn mul(self, rhs: Self) -> Self::Output { Value(self.0 * rhs.0) }
// }
//
// impl Div for &Value {
//     type Output = Value;
//     fn div(self, rhs: Self) -> Self::Output { Value(self.0 / rhs.0) }
// }

impl Value {
    pub fn number(value: f64) -> Value {
        Value {
            value: OrderedFloat(value),
            units: Factors::new(),
        }
    }
    pub fn with_unit(value: f64, units: Factors) -> Value {
        Value {
            value: OrderedFloat(value),
            units,
        }
    }
    pub fn value(&self) -> f64 { *self.value }
    pub fn units(&self) -> &Factors { &self.units }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}", self.value.0, self.units)
    }
}

// #[derive(Copy, Clone, Hash, Debug, Eq, PartialEq, Ord, PartialOrd)]
// pub struct Unit(pub Ustr);
//
//
// #[derive(Debug, Eq, PartialEq, Clone)]
// pub struct Value {
//     real: OrderedFloat<f64>,
//     dimension: Dimension,
// }
//
//
// impl ValueCtx {
//     pub fn new() -> Self {
//         ValueCtx { unit_ctx: HashMap::new() }
//     }
//     pub fn mul(&self, left: &Value, right: &Value) -> Value {
//         Value { real: left.real * right.real, dimension }
//     }
//     pub fn div(&self, left: &Value, right: &Value) -> Value { todo!() }
//     pub fn add(&self, left: &Value, right: &Value) -> Value { todo!() }
//     pub fn sub(&self, left: &Value, right: &Value) -> Value { todo!() }
// }
//
