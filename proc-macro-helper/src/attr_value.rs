use proc_macro2::{Ident, TokenStream as TokenStream2};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse2, Error, Expr, ExprLit, Lit, LitInt, Path, Result};

pub trait AttrValue: Sized {
    fn from_tokens(input: TokenStream2) -> Result<Self>;
}

impl AttrValue for Expr {
    fn from_tokens(input: TokenStream2) -> Result<Self> { parse2(input) }
}

impl AttrValue for Path {
    fn from_tokens(input: TokenStream2) -> Result<Self> { parse2(input) }
}

impl AttrValue for usize {
    fn from_tokens(input: TokenStream2) -> Result<Self> {
        <LitInt>::from_tokens(input)?.base10_parse()
    }
}

impl AttrValue for LitInt {
    fn from_tokens(input: TokenStream2) -> Result<Self> {
        match Lit::from_tokens(input)? {
            Lit::Int(int) => Ok(int),
            expr => Err(Error::new(expr.span(), "expected Lit::Int"))?,
        }
    }
}

impl AttrValue for Lit {
    fn from_tokens(input: TokenStream2) -> Result<Self> { Ok(ExprLit::from_tokens(input)?.lit) }
}

impl AttrValue for ExprLit {
    fn from_tokens(input: TokenStream2) -> Result<Self> {
        match Expr::from_tokens(input)? {
            Expr::Lit(lit) => Ok(lit),
            expr => Err(Error::new(expr.span(), "expected Expr::Lit"))?,
        }
    }
}

impl AttrValue for String {
    fn from_tokens(input: TokenStream2) -> Result<Self> {
        match Lit::from_tokens(input)? {
            Lit::Str(x) => Ok(x.value()),
            expr => Err(Error::new(expr.span(), "expected Lit::Str"))?,
        }
    }
}

impl<T: AttrValue> AttrValue for Vec<T> {
    fn from_tokens(input: TokenStream2) -> Result<Self> {
        fn parser<T: AttrValue>(stream: ParseStream) -> Result<Vec<T>> {
            Ok(Punctuated::<TokenStream2, Comma>::parse_terminated(stream)?
                .into_iter()
                .map(|x| T::from_tokens(x))
                .collect::<Result<_>>()?)
        }
        Parser::parse2(parser::<T>, input)
    }
}

impl AttrValue for Ident {
    fn from_tokens(input: TokenStream2) -> Result<Self> { parse2(input) }
}
