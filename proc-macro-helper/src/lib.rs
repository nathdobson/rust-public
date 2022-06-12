#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(unused_must_use)]
#![feature(try_blocks)]
#![feature(box_syntax)]
#![feature(local_key_cell_methods)]
#![feature(panic_info_message)]
#![feature(backtrace)]
#![feature(backtrace_frames)]
#![feature(never_type)]
#![feature(once_cell)]

extern crate proc_macro;

use std::any::Any;
use std::backtrace::Backtrace;
use std::cell::Cell;
use std::collections::HashMap;
use std::env::var;
use std::fmt::{Arguments, Debug, Display, Write};
use std::panic::{catch_unwind, resume_unwind, set_hook, AssertUnwindSafe};

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, parse_quote_spanned, AngleBracketedGenericArguments, Attribute,
    Error, Expr, GenericArgument, GenericParam, Generics, Item, ItemEnum, ItemStruct,
    PathArguments, Result, Token, Type,
};
use crate::attr_table::{parse_attr_table_from_tokens, ParseAttrTable};

use crate::attr_group::{AttrGroup, ParseAttrGroup};
use crate::helper::HelperItem;

pub mod attr_table;
pub mod attr_value;
pub mod attr_group;
pub mod helper;
#[cfg(test)]
mod tests;

pub fn proc_macro_derive_shim<A: ParseAttrGroup>(
    input: TokenStream,
    imp: impl FnOnce(A, HelperItem) -> Result<TokenStream2>,
) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    catch_unwind_as_compiler_error(|| {
        let helper = HelperItem::new(item)?;
        let attrs = AttrGroup::new(helper.item.span(), &helper.attrs())?;
        Result::Ok(imp(A::parse_attr_group(&attrs)?, helper)?)
    })
    .unwrap_or_else(|x| {
        Error::new(x.span(), format_args!("error from proc macro: {}", x)).into_compile_error()
    })
    .into()
}

pub fn proc_macro_attr_shim<A: ParseAttrTable>(
    attrs: TokenStream,
    input: TokenStream,
    imp: impl FnOnce(A, HelperItem) -> Result<TokenStream2>,
) -> TokenStream {
    let item: Item = parse_macro_input!(input as Item);
    catch_unwind_as_compiler_error(|| {
        let attrs = parse_attr_table_from_tokens(attrs.into())?;
        Result::Ok(imp(attrs, HelperItem::new(item)?)?)
    })
    .unwrap_or_else(|x| {
        Error::new(x.span(), format_args!("error from proc macro: {}", x)).into_compile_error()
    })
    .into()
}

pub fn catch_unwind_as_compiler_error<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    thread_local! {
        static LAST_PANIC: Cell<String> = Cell::default();
    }
    set_hook(box |panic_info| {
        let backtrace = Backtrace::force_capture();
        let message: &dyn Display = panic_info.message().map_or(&"<>", |x| x);
        let location: &dyn Display = panic_info.location().map_or(&"<>", |x| x);
        let mut panic = String::new();
        writeln!(panic, "{}", message).unwrap();
        writeln!(panic, "--> {}", location).unwrap();
        writeln!(panic, "{}", backtrace).unwrap();
        LAST_PANIC.set(panic);
    });
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|e| {
        Err(Error::new(
            Span::def_site(),
            format_args!("{}", LAST_PANIC.take()),
        ))
    })
}

fn pretty_print_item(item: Item) -> String {
    let file = syn::File {
        attrs: vec![],
        items: vec![item],
        shebang: None,
    };
    prettyplease::unparse(&file)
}

#[track_caller]
pub fn assert_equiv(x: Item, y: Item) {
    let x = pretty_print_item(x);
    let y = pretty_print_item(y);
    assert_eq!(x, y);
}
