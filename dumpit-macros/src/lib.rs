use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Fields, Index, Lit, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

// ---------------------------------------------------------------------------
// Attribute model
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FieldAttr {
    skip: bool,
    skip_if: Option<String>,
    format_str: Option<String>,
    format_args: Vec<String>,
    literal: Option<Lit>,
    with: Option<String>,
    take: Option<Lit>,
    truncate: Option<usize>,
}

struct FormatArgs {
    fmt: String,
    args: Vec<String>,
}

impl Parse for FormatArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit: syn::LitStr = input.parse()?;
        let mut args = Vec::new();
        while input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }
            let expr: Expr = input.parse()?;
            args.push(quote!(#expr).to_string());
        }
        Ok(FormatArgs {
            fmt: lit.value(),
            args,
        })
    }
}

fn parse_field_attrs(attrs: &[syn::Attribute]) -> syn::Result<FieldAttr> {
    let mut fa = FieldAttr::default();
    for attr in attrs {
        if !attr.path().is_ident("dump") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                fa.skip = true;
                return Ok(());
            }
            if meta.path.is_ident("skip_if") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: syn::LitStr = content.parse()?;
                    fa.skip_if = Some(lit.value());
                } else {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: syn::LitStr = meta.input.parse()?;
                    fa.skip_if = Some(lit.value());
                }
                return Ok(());
            }
            if meta.path.is_ident("format") {
                let content;
                syn::parenthesized!(content in meta.input);
                let parsed: FormatArgs = content.parse()?;
                fa.format_str = Some(parsed.fmt);
                fa.format_args = parsed.args;
                return Ok(());
            }
            if meta.path.is_ident("literal") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: Lit = content.parse()?;
                    fa.literal = Some(lit);
                } else {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: Lit = meta.input.parse()?;
                    fa.literal = Some(lit);
                }
                return Ok(());
            }
            if meta.path.is_ident("with") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: syn::LitStr = content.parse()?;
                    fa.with = Some(lit.value());
                } else {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: syn::LitStr = meta.input.parse()?;
                    fa.with = Some(lit.value());
                }
                return Ok(());
            }
            if meta.path.is_ident("take") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: Lit = content.parse()?;
                    fa.take = Some(lit);
                } else {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: Lit = meta.input.parse()?;
                    fa.take = Some(lit);
                }
                return Ok(());
            }
            if meta.path.is_ident("truncate") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let lit: syn::LitInt = content.parse()?;
                    fa.truncate = Some(lit.base10_parse()?);
                } else {
                    let _: Token![=] = meta.input.parse()?;
                    let lit: syn::LitInt = meta.input.parse()?;
                    fa.truncate = Some(lit.base10_parse()?);
                }
                return Ok(());
            }
            Err(meta.error("unrecognized dump attribute"))
        })?;
    }
    Ok(fa)
}

// ---------------------------------------------------------------------------
// Value expression generation
// ---------------------------------------------------------------------------

/// Generate the complete field call statement including any let-bindings
/// needed for temporaries. `emit_field` receives a `&dyn Debug` expression.
fn field_debug_token(
    attr: &FieldAttr,
    access: &TokenStream2,
    field_name: Option<&str>,
    emit_field: impl FnOnce(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    if attr.skip {
        return quote! {};
    }

    // take needs special handling: the field name becomes "name(n/total)"
    let body = if let Some(n) = &attr.take {
        if let Some(name) = field_name {
            quote! {
                {
                    let __take_val = ::dumpit::TakeIter(#access, #n);
                    let __field_name = __take_val.field_name(#name);
                    __ds.field(&__field_name, &__take_val as &dyn ::core::fmt::Debug);
                }
            }
        } else {
            // unnamed field (tuple struct/variant) — no name to attach count to
            let bindings = quote! {
                let __take_val = ::dumpit::TakeIter(#access, #n);
            };
            let val_ref = quote! { &__take_val as &dyn ::core::fmt::Debug };
            let field_call = emit_field(val_ref);
            quote! {
                {
                    #bindings
                    #field_call
                }
            }
        }
    } else {
        let (bindings, val_ref) = field_value_parts(attr, access);
        let field_call = emit_field(val_ref);
        quote! {
            {
                #bindings
                #field_call
            }
        }
    };

    if let Some(cond) = &attr.skip_if {
        let cond_expr: Expr = syn::parse_str(cond).expect("invalid skip_if expression");
        quote! {
            if !(#cond_expr) {
                #body
            }
        }
    } else {
        body
    }
}

/// Returns (binding_statements, value_ref_expr) where value_ref_expr is
/// something like `&__val as &dyn Debug` that can be used in `.field()`.
/// The binding_statements must appear before value_ref_expr is used.
fn field_value_parts(attr: &FieldAttr, access: &TokenStream2) -> (TokenStream2, TokenStream2) {
    if let Some(lit) = &attr.literal {
        return (quote! {}, quote! { &#lit as &dyn ::core::fmt::Debug });
    }
    if let Some(func_path) = &attr.with {
        let path: syn::ExprPath =
            syn::parse_str(func_path).expect("invalid function path in #[dump(with)]");
        let bindings = quote! {
            let __with = ::dumpit::WithFn(#access, #path);
        };
        return (bindings, quote! { &__with as &dyn ::core::fmt::Debug });
    }
    if let Some(fmt_str) = &attr.format_str {
        let extra: Vec<TokenStream2> = attr
            .format_args
            .iter()
            .map(|a| {
                let expr: Expr = syn::parse_str(a).expect("invalid format arg");
                quote! { , #expr }
            })
            .collect();
        let bindings = quote! {
            let __fmt = ::dumpit::Formatted(::std::format!(#fmt_str #(#extra)*));
        };
        return (bindings, quote! { &__fmt as &dyn ::core::fmt::Debug });
    }
    if attr.take.is_some() {
        // take is handled specially in field_debug_token
        unreachable!("take should be handled before field_value_parts");
    }
    if let Some(limit) = &attr.truncate {
        let bindings = quote! {
            use ::dumpit::DebugFallbackBuild as _;
            let __wrap = ::dumpit::TruncateWrap(#access, #limit);
            let __val = __wrap.__dumpit_build();
        };
        return (bindings, quote! { &__val as &dyn ::core::fmt::Debug });
    }
    // default — autoref specialization
    let bindings = quote! {
        use ::dumpit::DebugFallbackBuild as _;
        let __wrap = ::dumpit::DebugWrap(#access);
        let __val = __wrap.__dumpit_build();
    };
    (bindings, quote! { &__val as &dyn ::core::fmt::Debug })
}

// ---------------------------------------------------------------------------
// Derive entry point
// ---------------------------------------------------------------------------

/// The main entry point for the #[derive(Dump)] macro.
///
/// supported attributes:
/// - `#[dump(skip)]` — skip this field entirely
/// - `#[dump(skip_if = "expr")]` — skip if the expression evaluates to true (expr can refer to `self` and field names)
/// - `#[dump(format = "fmt", arg1, arg2, ...)]` — use a custom format string with optional arguments.
/// - `#[dump(literal = "value")]` — ignore the field value and print the literal instead
/// - `#[dump(with = "path::to::func")]` — use a custom function to format the field. The function should have signature `fn(&T, &mut Formatter) -> fmt::Result` where T is the field type.
/// - `#[dump(take = n)]` — for iterable fields, only include the first n items and show the count as "name(n/total)"
/// - `#[dump(truncate = n)]` — debug-format the field value, then truncate the output to n characters (adding "..." if truncated).
#[proc_macro_derive(Dump, attributes(dump))]
pub fn derive_dump(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data_struct) => generate_struct_body(name, &data_struct.fields),
        Data::Enum(data_enum) => {
            let arms: Vec<_> = data_enum
                .variants
                .iter()
                .map(|v| generate_enum_arm(name, &v.ident, &v.fields))
                .collect();
            quote! {
                match self {
                    #(#arms),*
                }
            }
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "Dump cannot be derived for unions")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::core::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    };
    expanded.into()
}

// ---------------------------------------------------------------------------
// Struct codegen
// ---------------------------------------------------------------------------

fn generate_struct_body(name: &syn::Ident, fields: &Fields) -> TokenStream2 {
    let name_str = name.to_string();
    match fields {
        Fields::Named(named) => {
            let field_calls: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let attr = parse_field_attrs(&f.attrs).expect("invalid dump attr");
                    let ident = f.ident.as_ref().unwrap();
                    let ident_str = ident.to_string();
                    let access = quote! { &self.#ident };
                    field_debug_token(&attr, &access, Some(&ident_str), |val| {
                        quote! { __ds.field(#ident_str, #val); }
                    })
                })
                .collect();
            quote! {
                let mut __ds = f.debug_struct(#name_str);
                #(#field_calls)*
                __ds.finish()
            }
        }
        Fields::Unnamed(unnamed) => {
            let field_calls: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let attr = parse_field_attrs(&f.attrs).expect("invalid dump attr");
                    let idx = Index::from(i);
                    let access = quote! { &self.#idx };
                    field_debug_token(&attr, &access, None, |val| {
                        quote! { __dt.field(#val); }
                    })
                })
                .collect();
            quote! {
                let mut __dt = f.debug_tuple(#name_str);
                #(#field_calls)*
                __dt.finish()
            }
        }
        Fields::Unit => {
            quote! { f.write_str(#name_str) }
        }
    }
}

// ---------------------------------------------------------------------------
// Enum codegen
// ---------------------------------------------------------------------------

fn generate_enum_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &Fields,
) -> TokenStream2 {
    let variant_str = variant_name.to_string();
    match fields {
        Fields::Named(named) => {
            let field_idents: Vec<_> = named
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap())
                .collect();
            let field_calls: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let attr = parse_field_attrs(&f.attrs).expect("invalid dump attr");
                    let ident = f.ident.as_ref().unwrap();
                    let ident_str = ident.to_string();
                    let access = quote! { #ident };
                    field_debug_token(&attr, &access, Some(&ident_str), |val| {
                        quote! { __ds.field(#ident_str, #val); }
                    })
                })
                .collect();
            quote! {
                #enum_name::#variant_name { #(#field_idents),* } => {
                    let mut __ds = f.debug_struct(#variant_str);
                    #(#field_calls)*
                    __ds.finish()
                }
            }
        }
        Fields::Unnamed(unnamed) => {
            let bindings: Vec<syn::Ident> = (0..unnamed.unnamed.len())
                .map(|i| syn::Ident::new(&format!("__field{}", i), proc_macro2::Span::call_site()))
                .collect();
            let field_calls: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let attr = parse_field_attrs(&f.attrs).expect("invalid dump attr");
                    let binding = &bindings[i];
                    let access = quote! { #binding };
                    field_debug_token(&attr, &access, None, |val| {
                        quote! { __dt.field(#val); }
                    })
                })
                .collect();
            quote! {
                #enum_name::#variant_name(#(#bindings),*) => {
                    let mut __dt = f.debug_tuple(#variant_str);
                    #(#field_calls)*
                    __dt.finish()
                }
            }
        }
        Fields::Unit => {
            quote! {
                #enum_name::#variant_name => {
                    f.write_str(#variant_str)
                }
            }
        }
    }
}
