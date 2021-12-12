#![feature(default_free_fn)]
#![allow(unused_imports)]
#![allow(unused_variables)]

extern crate proc_macro;

use proc_macro::{TokenStream};
use std::collections::hash_map::DefaultHasher;
use std::default::default;
use std::hash::{Hash, Hasher};
use proc_macro2::{Ident, Span};

use syn::{parse_macro_input, DeriveInput, Data, LitByteStr, Type, ItemFn, parse_quote, ReturnType, NestedMeta, Meta, Error, Lit, ItemStatic};
use quote::quote;
use syn::__private::TokenStream2;
use syn::Item;
use syn::ReturnType::Default;
use syn::AttributeArgs;
use syn::spanned::Spanned;

fn ctor(name: &Ident, body: &TokenStream2) -> TokenStream2 {
    let mut hasher = DefaultHasher::new();
    name.to_string().hash(&mut hasher);
    name.span().source_file().path().hash(&mut hasher);
    name.span().start().line.hash(&mut hasher);
    name.span().start().column.hash(&mut hasher);
    name.span().end().line.hash(&mut hasher);
    name.span().end().column.hash(&mut hasher);
    let hash = format!("{:016X}", hasher.finish());
    let pub_ident_fn =
        syn::parse_str::<syn::Ident>(format!("rust_registry___registry___{}", hash).as_ref())
            .expect("Unable to create identifier");
    let pub_ident_static =
        syn::parse_str::<syn::Ident>(format!("RUST_REGISTRY___REGISTRY___{}", hash).as_ref())
            .expect("Unable to create identifier");
    let bytes = format!("{} ", pub_ident_fn).into_bytes();
    let bytes = LitByteStr::new(&bytes, Span::call_site());
    quote! {
        cfg_if::cfg_if!(
            if #[cfg(any(target_arch = "wasm32", target_arch = "wasi"))] {
                #[registry::reexport::wasm_bindgen]
                #[doc(hidden)]
                pub fn #pub_ident_fn()  { #body }

                #[used]
                #[link_section = "registry_ctors"]
                #[no_mangle]
                #[doc(hidden)]
                pub static #pub_ident_static: [u8; #bytes.len()] = *#bytes;

            } else {
                #[registry::reexport::ctor]
                fn #pub_ident_fn() { #body }
            }
        );
    }
}

#[proc_macro_attribute]
pub fn register(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Item);
    let args = parse_macro_input!(args as AttributeArgs);
    match register_impl(args, input) {
        Ok(x) => x.into(),
        Err(x) => x.to_compile_error().into(),
    }
}

fn register_impl(args: AttributeArgs, input: Item) -> Result<TokenStream2, Error> {
    let mut registry = None;
    let mut lazy = false;
    for arg in &args {
        match arg {
            NestedMeta::Meta(meta) => {
                match meta {
                    Meta::Path(x) => registry = Some(x),
                    Meta::List(x) => return Err(Error::new(x.span(), "Cannot use List as arg")),
                    Meta::NameValue(namevalue) => {
                        if namevalue.path.segments.len() == 1 && namevalue.path.segments[0].ident.to_string() == "lazy" {
                            match &namevalue.lit {
                                Lit::Bool(v) => lazy = v.value,
                                _ => return Err(Error::new(namevalue.span(), "Argument to `lazy` must be bool")),
                            }
                        } else {
                            return Err(Error::new(namevalue.span(), "The only valid key is `lazy`"));
                        }
                    }
                }
            }
            NestedMeta::Lit(x) => return Err(Error::new(x.span(), "Cannot use List as arg")),
        }
    }
    let registry = registry.ok_or_else(|| Error::new(input.span(), "Must specify registry"))?;
    match &input {
        Item::Fn(f) => {
            let name = &f.sig.ident;
            let ctored = ctor(name, &quote! {
                ::registry::Registry::register(&#registry, |x| {
                    ::registry::BuilderFrom::insert(x, #name());
                })
            });
            Ok(quote! {
                #f
                #ctored
            })
        }
        Item::Static(s) => {
            if lazy {
                let ItemStatic {
                    attrs,
                    vis,
                    static_token,
                    mutability,
                    ident,
                    colon_token,
                    ty,
                    eq_token,
                    expr,
                    semi_token
                } = s;
                if let Some(mutability) = mutability {
                    return Err(Error::new(mutability.span(), "Cannot use mutable statics."));
                }
                let ctored = ctor(&ident, &quote! {
                    ::registry::Registry::register(&#registry, |x| {
                        ::registry::BuilderFrom::insert(x, ::registry::LazyEntry::__private(&#ident))
                    })
                });
                Ok(quote! {
                    #ctored
                    #( #attrs )*
                    #vis #static_token #ident #colon_token ::registry::LazyEntry<#ty> #eq_token
                        ::registry::LazyEntry::new(
                            || #expr,
                            || {
                                ::std::mem::drop(::std::ops::Deref::deref(&#registry));
                                ::registry::LazyEntry::__private(&#ident)
                            }
                        )
                     #semi_token
                })
            } else {
                let name = &s.ident;
                let ctored = ctor(&s.ident, &quote! {
                    ::registry::Registry::register(&#registry, |x| {
                        ::registry::BuilderFrom::insert(x, &#name)
                    })
                });
                Ok(quote! {
                    #ctored
                    #s
                })
            }
        }
        _ => return Err(Error::new(input.span(), "Macro only applies to functions and statics.")),
    }
}