use std::collections::{BTreeMap, HashMap};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, IdentFragment};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Paren;
use syn::{
    parenthesized, parse2, parse_quote, Attribute, Error, Expr, ExprLit, Item, Lit, LitInt, Result,
    Token,
};

use crate::attr_value::AttrValue;

#[derive(Debug)]
pub enum AttrKey {
    Crat(Token![crate]),
    Ident(Ident),
}

#[derive(Debug)]
pub struct AttrEntry {
    pub key: AttrKey,
    pub eq_token: Token![=],
    pub value: TokenStream2,
}

#[derive(Debug)]
pub struct AttrStream {
    span: Span,
    map: BTreeMap<String, AttrEntry>,
}

pub struct AttrParens {
    parens: Paren,
    stream: AttrStream,
}

#[derive(Debug)]
pub struct AttrGroup<'a> {
    span: Span,
    map: BTreeMap<String, &'a Attribute>,
}

impl Default for AttrStream {
    fn default() -> Self {
        AttrStream {
            span: Span::call_site(),
            map: Default::default(),
        }
    }
}

impl Spanned for AttrKey {
    fn span(&self) -> Span {
        match self {
            AttrKey::Crat(x) => x.span(),
            AttrKey::Ident(x) => x.span(),
        }
    }
}

impl Parse for AttrKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![crate]) {
            Ok(AttrKey::Crat(input.parse::<Token![crate]>()?))
        } else {
            Ok(AttrKey::Ident(input.parse::<Ident>()?))
        }
    }
}

impl Parse for AttrEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(AttrEntry {
            key: input.parse()?,
            eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl Parse for AttrStream {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let mut map = BTreeMap::new();
        for entry in Punctuated::<AttrEntry, Token![,]>::parse_terminated(input)? {
            let name = match &entry.key {
                AttrKey::Crat(_) => "crate".to_string(),
                AttrKey::Ident(i) => i.to_string(),
            };
            map.insert(name, entry);
        }
        Ok(AttrStream { span, map })
    }
}

impl Parse for AttrParens {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(AttrParens {
            parens: parenthesized!(content in input),
            stream: AttrStream::parse(&content)?,
        })
    }
}

impl AttrStream {
    pub fn new(attr: &Attribute) -> Result<Self> {
        if attr.tokens.is_empty() {
            let attr_path = &attr.path;
            return Err(Error::new(
                attr.path.span(),
                format_args!("Expected {}(...)", quote!(#attr_path)),
            ));
        }
        Ok(syn::parse2::<AttrParens>(attr.tokens.clone())?.stream)
    }
    pub fn parse_entry<T: AttrValue>(&mut self, name: &str) -> Result<T> {
        Ok(self
            .parse_entry_option::<T>(name)?
            .ok_or_else(|| Error::new(self.span, format_args!("Expected {} = ...", name)))?)
    }
    pub fn parse_entry_option<T: AttrValue>(&mut self, name: &str) -> Result<Option<T>> {
        if let Some(e) = self.map.remove(name) {
            Ok(Some(T::from_tokens(e.value.clone())?))
        } else {
            Ok(None)
        }
    }
    pub fn finish(self) -> Result<()> {
        if let Some((key, value)) = self.map.iter().next() {
            return Err(Error::new(
                value.key.span(),
                format_args!("Unexpected key `{}'", key),
            ));
        }
        Ok(())
    }
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
    pub fn parse_option<T: ParseAttr>(&self, key: &str) -> Result<Option<T>> {
        if let Some(attr) = self.map.get(key) {
            let mut stream = AttrStream::new(attr)?;
            let result = T::parse_attr(&mut stream)?;
            stream.finish()?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
    pub fn parse<T: ParseAttr>(&self, key: &str) -> Result<T> {
        Ok(self
            .parse_option(key)?
            .ok_or_else(|| Error::new(self.span, format_args!("Expected #[{}(...)]", key)))?)
    }
}

pub trait ParseAttr: Sized {
    fn parse_attr(stream: &mut AttrStream) -> Result<Self>;
}

pub trait ParseAttrGroup: Sized {
    fn parse(group: &AttrGroup) -> Result<Self>;
}

impl ParseAttrGroup for () {
    fn parse(group: &AttrGroup) -> Result<Self> { Ok(()) }
}
