use std::fmt::{Debug, Formatter};
use ustr::Ustr;

pub struct Block(pub Vec<Stmt>);

pub enum Stmt {
    Let(Ustr, Box<Expr>),
    Return(Box<Expr>),
}

#[derive(Clone)]
pub struct UnitPart(pub Vec<(Ustr, i32)>);

#[derive(Clone)]
pub enum UnitSet {
    None,
    Simple(UnitPart),
    Fraction(UnitPart, UnitPart),
}

#[derive(Clone)]
pub enum Expr {
    Number(f64),
    Var(Ustr),
    Op(Box<Expr>, Opcode, Box<Expr>),
    AsUnits(Box<Expr>, Vec<UnitSet>),
    Call(Ustr, Vec<Box<Expr>>),
    WithUnits(Box<Expr>, UnitSet),
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Opcode {
    Mul,
    Div,
    Add,
    Sub,
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number(x) =>
                write!(f, "{:?}", x),
            Expr::Op(a, op, b) =>
                write!(f, "({:?} {:?} {:?})", a, op, b),
            Expr::Var(x) =>
                write!(f, "{}", x),
            Expr::Call(fun, es) =>
                write!(f, "{:?}({:?})", fun, es),
            Expr::WithUnits(e, u) =>
                write!(f, "{:?} {:?}", e, u),
            Expr::AsUnits(e, u) =>
                write!(f, "{:?} {:?}", e, u),
        }
    }
}

impl Debug for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Mul => write!(f, "*"),
            Opcode::Div => write!(f, "/"),
            Opcode::Add => write!(f, "+"),
            Opcode::Sub => write!(f, "-"),
        }
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in self.0.iter() {
            write!(f, "{:?}\n", x)?;
        }
        Ok(())
    }
}

impl Debug for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::Let(x, e) => write!(f, "let {} = {:?};", x, e),
            Stmt::Return(e) => write!(f, "return {:?} ;", e),
        }
    }
}

impl Debug for UnitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitSet::Simple(x) => write!(f, "{:?}", x),
            UnitSet::Fraction(x, y) => write!(f, "{:?} per {:?}", x, y),
            UnitSet::None => write!(f, "-"),
        }
    }
}

impl Debug for UnitPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}