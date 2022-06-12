#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_helper::attr_group::{AttrGroup, ParseAttrGroup};
use proc_macro_helper::attr_table::{AttrTableBuilder, ParseAttrTable};
use proc_macro_helper::helper::{HelperDefinition, HelperItem};
use proc_macro_helper::{assert_equiv, proc_macro_attr_shim, proc_macro_derive_shim};
use quote::quote;
use syn::parse::ParseStream;
use syn::{parse2, parse_quote, Error, Item};

struct MyAttrGroup {
    my_attr: MyAttr,
}

struct MyAttr {
    my_key1: usize,
    my_key2: usize,
}

impl ParseAttrGroup for MyAttrGroup {
    fn parse_attr_group(group: &AttrGroup) -> syn::Result<Self> {
        Ok(MyAttrGroup {
            my_attr: group.get_unwrap("my_attr")?,
        })
    }
}

impl ParseAttrTable for MyAttr {
    fn parse_attr_table<'a>(input: ParseStream) -> syn::Result<Self> {
        let mut table = AttrTableBuilder::new();
        let my_key1 = table.add::<usize>("my_key1");
        let my_key2 = table.add::<usize>("my_key2");
        let mut table = table.build(input)?;
        Ok(MyAttr {
            my_key1: table.take_unwrap(my_key1)?,
            my_key2: table.take_unwrap(my_key2)?,
        })
    }
}

#[proc_macro_derive(MyDerive, attributes(my_attr))]
pub fn my_derive(input: TokenStream) -> TokenStream {
    proc_macro_derive_shim(input, my_derive_impl)
}

fn my_derive_impl(attrs: MyAttrGroup, item: HelperItem) -> Result<TokenStream2, Error> {
    let value = attrs.my_attr.my_key1;
    Ok(quote! {
        const MY_DERIVE: usize = #value;
    })
}

#[proc_macro_attribute]
pub fn my_attr_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    proc_macro_attr_shim(args, input, my_attr_macro_impl)
}

fn my_attr_macro_impl(attrs: MyAttr, item: HelperItem) -> Result<TokenStream2, Error> {
    let value = attrs.my_key1;
    Ok(quote! {
        const MY_ATTR_MACRO: usize = #value;
    })
}

#[proc_macro_derive(MyClone)]
pub fn my_clone(input: TokenStream) -> TokenStream { proc_macro_derive_shim(input, my_clone_impl) }

fn my_clone_impl(attrs: (), item: HelperItem) -> Result<TokenStream2, Error> {
    let typ = item.ident();
    let generics = item.generics();
    let generic_args = item.generic_args();
    let inner_types = item.inner_types();
    let expr = match item.definition() {
        HelperDefinition::Struct(def) => def.build(|index| {
            let projector = def.projector(index);
            parse_quote! {
                ::std::clone::Clone::clone(&self.#projector)
            }
        }),
        HelperDefinition::Enum(def) => {
            def.do_match(parse_quote! {self}, "", |variant, field_names| {
                def.build(variant, |field| {
                    let field_name = &field_names[field];
                    parse_quote! {
                        ::std::clone::Clone::clone(#field_name)
                    }
                })
            })
        }
    };
    Ok(quote! {
        impl #generics Clone for #typ #generic_args where #(#inner_types: Clone),*{
            fn clone(&self) -> Self{
                #expr
            }
        }
    })
}

#[test]
fn my_clone_test() {
    #[track_caller]
    fn case(input: Item, expected: TokenStream2) {
        let output = my_clone_impl((), HelperItem::new(input).unwrap()).unwrap();
        assert_equiv(parse2(output).unwrap(), parse2(expected).unwrap());
    }
    case(
        parse_quote! {
            struct Foo;
        },
        quote! {
            impl Clone for Foo<> {
                fn clone(&self)->Self{
                    Foo
                }
            }
        },
    );
    case(
        parse_quote! {
            struct Foo(Bar);
        },
        quote! {
            impl Clone for Foo<> where Bar: Clone{
                fn clone(&self) -> Self {
                    Foo(::std::clone::Clone::clone(&self.0))
                }
            }
        },
    );
    case(
        parse_quote! {
            struct Foo { bar:Bar, baz:Baz }
        },
        quote! {
            impl Clone for Foo<>
            where
                Bar: Clone,
                Baz: Clone,
            {
                fn clone(&self) -> Self {
                    Foo {
                        bar: ::std::clone::Clone::clone(&self.bar),
                        baz: ::std::clone::Clone::clone(&self.baz),
                    }
                }
            }
        },
    );
    case(
        parse_quote! {
            struct Foo<A, B> { bar:Bar<A>, baz:Baz<B> }
        },
        quote! {
            impl<A, B> Clone for Foo<A, B>
            where
                Bar<A>: Clone,
                Baz<B>: Clone,
            {
                fn clone(&self) -> Self {
                    Foo {
                        bar: ::std::clone::Clone::clone(&self.bar),
                        baz: ::std::clone::Clone::clone(&self.baz),
                    }
                }
            }
        },
    );
    case(
        parse_quote! {
            enum Foo {
                Bar,
                Baz(Zab),
                Qux{xuq:Xuq},
            }
        },
        quote! {
            impl Clone for Foo<>
            where
                Zab: Clone,
                Xuq: Clone,
            {
                fn clone(&self) -> Self {
                    match self {
                        Foo::Bar => Foo::Bar,
                        Foo::Baz(pmh_0) => Foo::Baz(::std::clone::Clone::clone(pmh_0)),
                        Foo::Qux { xuq: pmh_xuq } => {
                            Foo::Qux {
                                xuq: ::std::clone::Clone::clone(pmh_xuq),
                            }
                        }
                    }
                }
            }
        },
    );
}
