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

use crate::attr_value::ParseAttrValue;

pub trait ParseAttrTable: Sized {
    fn parse_attr_table(stream: ParseStream) -> Result<Self>;
}

pub struct AttrTableKey<T>(&'static str, PhantomData<T>);

pub struct AttrTableBuilder {
    parsers:
        BTreeMap<&'static str, Box<dyn for<'b> FnOnce(ParseStream<'b>) -> Result<Box<dyn Any>>>>,
}

pub struct AttrTable {
    span: Span,
    entries: BTreeMap<&'static str, Box<dyn Any>>,
}

impl AttrTableBuilder {
    pub fn new() -> Self {
        AttrTableBuilder {
            parsers: BTreeMap::new(),
        }
    }
    pub fn add<T: ParseAttrValue + 'static>(&mut self, key: &'static str) -> AttrTableKey<T> {
        self.parsers
            .insert(key, box |stream| Ok(box T::parse_attr_value(stream)?));
        AttrTableKey(key, PhantomData)
    }
    pub fn build(mut self, stream: ParseStream) -> Result<AttrTable> {
        let span = stream.span();
        let mut entries = BTreeMap::new();
        while !stream.is_empty() {
            let name;
            let span: Span;
            if stream.peek(Crate) {
                let crat = stream.parse::<Crate>()?;
                span = crat.span();
                name = "crate".to_string();
            } else {
                let ident = stream.parse::<Ident>()?;
                name = ident.to_string();
                span = ident.span();
            }
            let (key, parser) = self
                .parsers
                .remove_entry(&*name)
                .ok_or(Error::new(span, format_args!("unrecognized key: {}", name)))?;
            stream.parse::<Eq>()?;
            let value = parser(stream)?;
            entries.insert(key, value);
            if stream.is_empty() {
                break;
            } else if stream.peek(Comma) {
                stream.parse::<Comma>()?;
            } else {
                return Err(Error::new(stream.span(), "unexpected token"));
            }
        }
        Ok(AttrTable { span, entries })
    }
}

impl AttrTable {
    pub fn take<T: 'static>(&mut self, key: AttrTableKey<T>) -> Option<T> {
        Some(*Box::<dyn Any>::downcast(self.entries.remove(key.0)?).expect("unrecognized key type"))
    }
    pub fn take_unwrap<T: 'static>(&mut self, key: AttrTableKey<T>) -> Result<T> {
        let key_name = key.0;
        self.take(key)
            .ok_or_else(|| Error::new(self.span, format_args!("missing key {}", key_name)))
    }
}

pub fn parse_attr_table_from_attribute<T: ParseAttrTable>(attribute: &Attribute) -> Result<T> {
    fn parser<T: ParseAttrTable>(stream: ParseStream) -> Result<T> {
        let content;
        parenthesized!(content in stream);
        T::parse_attr_table(&content)
    }
    Parser::parse2(parser, attribute.tokens.clone())
}

pub fn parse_attr_table_from_tokens<T: ParseAttrTable>(tokens: TokenStream) -> Result<T> {
    Parser::parse2(T::parse_attr_table, tokens)
}

pub fn parse_attr_table_from_default<T: ParseAttrTable>() -> T {
    Parser::parse2(T::parse_attr_table, TokenStream2::new()).unwrap()
}
