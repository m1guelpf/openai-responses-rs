use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, LitStr};

/// Concatenate `///` doc comments into one string.
fn collect_docs(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                // `#[doc = "…"]`
                attr.parse_args::<LitStr>()
                    .ok()
                    .map(|lit| lit.value().trim().to_owned())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract `name` / `description` from:
///
/// ```ignore
/// #[openai(name = "foo", description = "bar")]
/// ```
fn extract_openai_attrs(attrs: &[Attribute]) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut description = None;

    for attr in attrs {
        if attr.path().is_ident("openai") {
            // Syn 2.x API: parse_nested_meta lets us traverse key/value pairs.
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let lit: LitStr = meta.value()?.parse()?;
                    name = Some(lit.value());
                } else if meta.path.is_ident("description") {
                    let lit: LitStr = meta.value()?.parse()?;
                    description = Some(lit.value());
                }
                Ok(())
            });
        }
    }

    (name, description)
}

/// `#[derive(OpenAiFunction)]`
///
/// The macro injects
/// ```ignore
/// const NAME: &'static str;
/// const DESCRIPTION: &'static str;
/// impl openai_responses::tool_builder::IntoTool for MyStruct { /* … */ }
/// ```
#[proc_macro_derive(OpenAiFunction, attributes(openai))]
pub fn derive_openai_function(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let docs = collect_docs(&input.attrs);
    let (attr_name, attr_desc) = extract_openai_attrs(&input.attrs);

    let name_const = attr_name.unwrap_or_else(|| ident.to_string());
    let desc_const = attr_desc.unwrap_or(docs);

    let expanded = quote! {
        impl openai_responses::tool_builder::IntoFunction for #ident {
            const NAME: &'static str = #name_const;
            const DESCRIPTION: &'static str = #desc_const;
        }
    };

    TokenStream::from(expanded)
}
