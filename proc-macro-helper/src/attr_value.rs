use std::collections::HashSet;
use std::hash::Hash;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse2, Error, Expr, ExprLit, Lit, LitInt, LitStr, Path, Result};

pub trait ParseAttrValue: Sized {
    fn parse_attr_value(stream: ParseStream) -> Result<Self>;
}

impl ParseAttrValue for Expr {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { Self::parse(stream) }
}

impl ParseAttrValue for Path {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { Self::parse(stream) }
}

impl ParseAttrValue for usize {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> {
        LitInt::parse(stream)?.base10_parse()
    }
}

impl ParseAttrValue for LitInt {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { LitInt::parse(stream) }
}

impl ParseAttrValue for Lit {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { Lit::parse(stream) }
}

impl ParseAttrValue for ExprLit {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { ExprLit::parse(stream) }
}

impl ParseAttrValue for String {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> {
        Ok(<LitStr as Parse>::parse(stream)?.value())
    }
}

impl<T: ParseAttrValue> ParseAttrValue for Vec<T> {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> {
        Ok(
            Punctuated::<T, Comma>::parse_terminated_with(stream, |s2| T::parse_attr_value(s2))?
                .into_iter()
                .collect(),
        )
    }
}
impl<T: ParseAttrValue + Hash + Eq> ParseAttrValue for HashSet<T> {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> {
        Ok(
            Punctuated::<T, Comma>::parse_terminated_with(stream, |s2| T::parse_attr_value(s2))?
                .into_iter()
                .collect(),
        )
    }
}

impl ParseAttrValue for Ident {
    fn parse_attr_value(stream: ParseStream) -> Result<Self> { Ident::parse(stream) }
}
