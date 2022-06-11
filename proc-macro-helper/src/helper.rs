use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use syn::{AngleBracketedGenericArguments, Arm, Attribute, AttributeArgs, ConstParam, Error, Expr, ExprCall, ExprField, ExprMatch, ExprPath, ExprStruct, FieldPat, Fields, FieldValue, GenericArgument, GenericParam, Generics, Index, Item, ItemEnum, ItemStruct, LifetimeDef, Member, parse_quote, Pat, Path, PatIdent, PatStruct, Type, TypeParam, Variant};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::Result;
use syn::token::Comma;

#[derive(Debug)]
pub struct HelperEnum {
    item: ItemEnum,
}

#[derive(Debug)]
pub struct HelperStruct {
    item: ItemStruct,
}

#[derive(Debug)]
pub enum HelperDefinition {
    Enum(HelperEnum),
    Struct(HelperStruct),
}

#[derive(Debug)]
pub struct HelperItem {
    pub item: Item,
}

fn item_sort(item: &Item) -> &'static str {
    match item {
        Item::Const(_) => "const",
        Item::Enum(_) => "enum",
        Item::ExternCrate(_) => "extern crate",
        Item::Fn(_) => "fn",
        Item::ForeignMod(_) => "extern \"C\"",
        Item::Impl(_) => "impl",
        Item::Macro(_) => "macro_rules",
        Item::Macro2(_) => "macro",
        Item::Mod(_) => "mod",
        Item::Static(_) => "static",
        Item::Struct(_) => "struct",
        Item::Trait(_) => "trait",
        Item::TraitAlias(_) => "trait alias",
        Item::Type(_) => "type",
        Item::Union(_) => "union",
        Item::Use(_) => "use",
        Item::Verbatim(_) => "verbatim",
        _ => "<unknown>",
    }
}

fn handle_unknown_item(item: &Item) -> Result<!> {
    Result::Err(Error::new(item.span(), format_args!("unsupported item {}", item_sort(item))))
}

impl HelperItem {
    pub fn ident(&self) -> &Ident {
        match &self.item {
            Item::Struct(s) => &s.ident,
            Item::Enum(e) => &e.ident,
            _ => unreachable!(),
        }
    }
    pub fn attrs(&self) -> &Vec<Attribute> {
        match &self.item {
            Item::Struct(s) => &s.attrs,
            Item::Enum(e) => &e.attrs,
            _ => unreachable!(),
        }
    }
    pub fn generics(&self) -> &Generics {
        match &self.item {
            Item::Struct(s) => &s.generics,
            Item::Enum(e) => &e.generics,
            _ => unreachable!(),
        }
    }
    pub fn generic_args(&self) -> AngleBracketedGenericArguments {
        let generics = self.generics();
        let mut generic_args = Punctuated::new();
        for x in generics.params.pairs() {
            match x.value() {
                GenericParam::Type(t) => {
                    let ident = &t.ident;
                    generic_args.push(parse_quote!(#ident));
                }
                GenericParam::Lifetime(l) => {
                    let lt = &l.lifetime;
                    generic_args.push(parse_quote!(#l));
                }
                GenericParam::Const(c) => todo!(),
            }
            if let Some(p) = x.punct() {
                generic_args.push_punct(**p);
            }
        }
        AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: generics.lt_token.unwrap_or_default(),
            args: generic_args,
            gt_token: generics.gt_token.unwrap_or_default(),
        }
    }
    pub fn generic_types(&self) -> Vec<&TypeParam> {
        let mut generic_types = vec![];
        for x in self.generics().params.pairs() {
            match x.value() {
                GenericParam::Type(t) => {
                    generic_types.push(t);
                }
                _ => {}
            }
        }
        generic_types
    }
    pub fn generic_consts(&self) -> Vec<&ConstParam> {
        let mut generic_types = vec![];
        for x in self.generics().params.pairs() {
            match x.value() {
                GenericParam::Const(t) => {
                    generic_types.push(t);
                }
                _ => {}
            }
        }
        generic_types
    }
    pub fn generic_lifetimes(&self) -> Vec<&LifetimeDef> {
        let mut generic_types = vec![];
        for x in self.generics().params.pairs() {
            match x.value() {
                GenericParam::Lifetime(t) => {
                    generic_types.push(t);
                }
                _ => {}
            }
        }
        generic_types
    }
    pub fn inner_types(&self) -> Vec<Type> {
        let mut inner_types = vec![];
        match &self.item {
            Item::Struct(s) => {
                for field in &s.fields {
                    inner_types.push(field.ty.clone());
                }
            }
            Item::Enum(e) => {
                for variant in &e.variants {
                    for field in &variant.fields {
                        inner_types.push(field.ty.clone())
                    }
                }
            }
            _ => unreachable!(),
        }
        inner_types
    }
    pub fn definition(&self) -> HelperDefinition {
        match &self.item {
            Item::Struct(s) => HelperDefinition::Struct(HelperStruct { item: s.clone() }),
            Item::Enum(e) => HelperDefinition::Enum(HelperEnum { item: e.clone() }),
            _ => unreachable!(),
        }
    }
    pub fn new(item: Item) -> Result<Self> {
        match item {
            Item::Struct(_) => {}
            Item::Enum(_) => {}
            _ => handle_unknown_item(&item)?,
        }
        Ok(HelperItem { item })
    }
}

impl HelperStruct {
    pub fn projector(&self, index: usize) -> Member {
        let field = self.item.fields.iter().nth(index).unwrap();
        if let Some(ident) = field.ident.clone() {
            Member::Named(ident)
        } else {
            Member::Unnamed(Index { index: index as u32, span: field.span() })
        }
    }
    pub fn project(&self, expr: Expr, index: usize) -> Expr {
        Expr::Field(ExprField {
            attrs: vec![],
            base: box expr,
            dot_token: Default::default(),
            member: self.projector(index),
        })
    }
    pub fn build(&self, mut builder: impl FnMut(usize) -> Expr) -> Expr {
        let path = &self.item.ident;
        match &self.item.fields {
            Fields::Named(named) => {
                let mut fields = Punctuated::new();
                for (index, field) in named.named.pairs().enumerate() {
                    fields.push(FieldValue {
                        attrs: vec![],
                        member: Member::Named(field.value().ident.clone().unwrap()),
                        colon_token: field.value().colon_token,
                        expr: builder(index),
                    });
                    if let Some(punct) = field.punct() {
                        fields.push_punct((*punct).clone())
                    }
                }
                Expr::Struct(ExprStruct {
                    attrs: vec![],
                    path: parse_quote!(#path),
                    brace_token: named.brace_token,
                    fields,
                    dot2_token: None,
                    rest: None,
                })
            }
            Fields::Unnamed(unnamed) => {
                let mut fields = Punctuated::new();
                for (index, field) in unnamed.unnamed.pairs().enumerate() {
                    fields.push(builder(index));
                    if let Some(punct) = field.punct() {
                        fields.push_punct((*punct).clone())
                    }
                }
                Expr::Call(ExprCall {
                    attrs: vec![],
                    func: parse_quote!(#path),
                    paren_token: unnamed.paren_token,
                    args: fields,
                })
            }
            Fields::Unit => {
                Expr::Path(parse_quote!(#path))
            }
        }
    }
}


impl HelperEnum {
    pub fn do_match(&self, expr: Expr, prefix: &str, mut arm_fn: impl FnMut(usize, Vec<Ident>) -> Expr) -> Expr {
        let prefix = if prefix.is_empty() {
            format!("")
        } else {
            format!("{}_", prefix)
        };
        let enum_name = &self.item.ident;
        let mut arms = vec![];
        for (variant_index, variant) in self.item.variants.pairs().enumerate() {
            let variant_name = &variant.value().ident;
            let pat: Pat;
            let mut vars: Vec<Ident> = vec![];
            match &variant.value().fields {
                Fields::Named(named) => {
                    let mut pats: Punctuated<FieldPat, Comma> = Punctuated::new();
                    for field in named.named.pairs() {
                        let field_name = field.value().ident.as_ref().unwrap();
                        let var = format_ident!("pmh_{}{}", prefix, field_name);
                        vars.push(var.clone());
                        pats.push(FieldPat {
                            attrs: vec![],
                            member: Member::Named(field_name.clone()),
                            colon_token: field.value().colon_token,
                            pat: box Pat::Ident(PatIdent {
                                attrs: vec![],
                                by_ref: None,
                                mutability: None,
                                ident: var,
                                subpat: None,
                            }),
                        });
                        if let Some(punct) = field.punct() {
                            pats.push_punct((*punct).clone());
                        }
                    }
                    pat = Pat::Struct(PatStruct {
                        attrs: vec![],
                        path: parse_quote!(#enum_name::#variant_name),
                        brace_token: named.brace_token,
                        fields: pats,
                        dot2_token: None,
                    });
                }
                Fields::Unnamed(unnamed) => {
                    for i in 0..unnamed.unnamed.len() {
                        vars.push(format_ident!("pmh_{}{}", prefix, i));
                    }
                    pat = parse_quote!(#enum_name::#variant_name(#(#vars),*));
                }
                Fields::Unit => {
                    pat = parse_quote!(#enum_name::#variant_name);
                }
            }
            arms.push(Arm {
                attrs: vec![],
                pat: pat,
                guard: None,
                fat_arrow_token: Default::default(),
                body: box arm_fn(variant_index, vars),
                comma: None,
            })
        }
        Expr::Match(ExprMatch {
            attrs: vec![],
            match_token: Default::default(),
            expr: box expr,
            brace_token: Default::default(),
            arms,
        })
    }
    pub fn build(&self, variant: usize, mut builder: impl FnMut(usize) -> Expr) -> Expr {
        let enum_name = &self.item.ident;
        let variant_name = &self.item.variants[variant].ident;
        match &self.item.variants[variant].fields {
            Fields::Named(named) => {
                let mut fields = Punctuated::new();
                for (index, field) in named.named.pairs().enumerate() {
                    fields.push(FieldValue {
                        attrs: vec![],
                        member: Member::Named(field.value().ident.clone().unwrap()),
                        colon_token: field.value().colon_token,
                        expr: builder(index),
                    });
                    if let Some(punct) = field.punct() {
                        fields.push_punct((*punct).clone())
                    }
                }
                Expr::Struct(ExprStruct {
                    attrs: vec![],
                    path: parse_quote! {#enum_name::# variant_name},
                    brace_token: named.brace_token,
                    fields,
                    dot2_token: None,
                    rest: None,
                })
            }
            Fields::Unnamed(unnamed) => {
                let mut fields = Punctuated::new();
                for (index, field) in unnamed.unnamed.pairs().enumerate() {
                    fields.push(builder(index));
                    if let Some(punct) = field.punct() {
                        fields.push_punct((*punct).clone())
                    }
                }
                Expr::Call(ExprCall {
                    attrs: vec![],
                    func: parse_quote! {#enum_name::# variant_name},
                    paren_token: unnamed.paren_token,
                    args: fields,
                })
            }
            Fields::Unit => {
                Expr::Path(parse_quote! {#enum_name::# variant_name})
            }
        }
    }
}