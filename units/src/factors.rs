use std::fmt::{Debug, Formatter};
use std::iter;
use std::iter::FromIterator;
use std::ops::{Div, Mul};

use ustr::{ustr, Ustr};

use crate::map::VecMap;
use crate::unicode::Superscript;

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Factors(VecMap<Ustr, i32>);

impl From<Ustr> for Factors {
    fn from(x: Ustr) -> Self { Factors(iter::once((x, 1)).collect()) }
}

impl From<&'static str> for Factors {
    fn from(x: &'static str) -> Self { Self::from(ustr(x)) }
}

impl<const N: usize> From<[(&'static str, i32); N]> for Factors {
    fn from(x: [(&'static str, i32); N]) -> Self {
        Factors(
            <[(&'static str, i32); N]>::into_iter(x)
                .map(|(k, v)| (ustr(k), v))
                .collect(),
        )
    }
}

impl FromIterator<(Ustr, i32)> for Factors {
    fn from_iter<T: IntoIterator<Item = (Ustr, i32)>>(iter: T) -> Self {
        Factors(VecMap::from_iter(iter))
    }
}

impl<'a> IntoIterator for &'a Factors {
    type Item = (&'a Ustr, &'a i32);
    type IntoIter = impl Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter { (&self.0).into_iter() }
}

impl Factors {
    pub fn new() -> Self { Factors(VecMap::new()) }
    pub fn powi(&self, p: i32) -> Self {
        if p == 0 {
            return Factors::new();
        } else {
            Factors(self.0.iter().map(|(k, v)| (*k, *v * p)).collect())
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Ustr, &i32)> { self.0.iter() }
}

impl Mul for &Factors {
    type Output = Factors;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut output = self.clone();
        for (key, count) in rhs.0.iter() {
            *output.0.entry(*key).or_default() += count;
        }
        output.0.retain(|_, c| *c != 0);
        output
    }
}

impl Div for &Factors {
    type Output = Factors;
    fn div(self, rhs: Self) -> Self::Output {
        let mut output = self.clone();
        for (key, count) in rhs.0.iter() {
            *output.0.entry(*key).or_default() -= count;
        }
        output.0.retain(|_, c| *c != 0);
        output
    }
}

impl Debug for Factors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut num = false;
        for (&u, &power) in self.0.iter() {
            if power >= 0 {
                if num {
                    write!(f, " ")?;
                }
                num = true;
                write!(f, "{}{:?}", u, Superscript(power))?;
            }
        }
        let mut denom = false;
        for (&u, &power) in self.0.iter() {
            if power < 0 {
                if !denom {
                    denom = true;
                    write!(f, " per ")?;
                } else {
                    write!(f, " ")?;
                }
                write!(f, "{}{:?}", u, Superscript(-power))?;
            }
        }
        Ok(())
    }
}
