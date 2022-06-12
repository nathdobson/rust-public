use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::lazy::{OnceCell, SyncOnceCell};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use proc_macro2::{Ident, Span, TokenStream, TokenStream as TokenStream2};
use quote::{quote, IdentFragment};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{At, Comma, Crate, Eq, Paren};
use syn::{
    parenthesized, parse2, parse_quote, Attribute, Error, Expr, ExprLit, Item, Lit, LitInt, Path,
    Result, Token,
};
use crate::attr_table::{parse_attr_table_from_attribute, ParseAttrTable};

use crate::attr_value::ParseAttrValue;

#[derive(Debug)]
pub struct AttrGroup<'a> {
    span: Span,
    map: BTreeMap<String, &'a Attribute>,
}

impl<'a> AttrGroup<'a> {
    pub fn new(span: Span, attrs: &'a [Attribute]) -> Result<Self> {
        let mut map = BTreeMap::new();
        for attr in attrs {
            let key = attr.path.segments.last().unwrap().ident.to_string();
            map.insert(key, attr);
        }
        Ok(AttrGroup { span, map })
    }
    pub fn get<T: ParseAttrTable>(&self, key: &str) -> Result<Option<T>> {
        if let Some(attr) = self.map.get(key) {
            Ok(Some(parse_attr_table_from_attribute(attr)?))
        } else {
            Ok(None)
        }
    }
    pub fn get_unwrap<T: ParseAttrTable>(&self, key: &str) -> Result<T> {
        Ok(self
            .get(key)?
            .ok_or_else(|| Error::new(self.span, format_args!("Expected #[{}(...)]", key)))?)
    }
}

pub trait ParseAttrGroup: Sized {
    fn parse_attr_group(group: &AttrGroup) -> Result<Self>;
}

impl ParseAttrGroup for () {
    fn parse_attr_group(group: &AttrGroup) -> Result<Self> { Ok(()) }
}
