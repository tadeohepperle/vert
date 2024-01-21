use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote, TypePath};

/// Derives the Lerp trait for a struct where each field implements Lerp.
/// For example the Struct:
/// ```rust,no-run
/// struct Color{
///     r: f32,
///     g: f32,
///     b: f32,
/// }
/// ```
///
/// Will get a lerp implementation that is:
///
/// ```rust,no-run
/// impl Lerp for Color{
///     fn lerp(&self, other: &Self, factor: f32) -> Self {
///         Color {
///             r: self.r.lerp(&other.r, factor),
///             g: self.g.lerp(&other.g, factor),
///             b: self.b.lerp(&other.b, factor),
///         }
///     }
/// }
/// ```
///
/// Don't use this Derive Macro if the fields should not be lerped independently.
#[proc_macro_derive(Lerp)]
pub fn derive_lerp(input: TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();
    let stru = match &derive_input.data {
        syn::Data::Struct(s) => s,
        _ => panic!("Only derive Dependencies on structs"),
    };
    let stru_ident = derive_input.ident;
    // let fields: Vec<TypePath>
    // s.fields.iter().map(|e|)

    let lerp_impl_body = match &stru.fields {
        syn::Fields::Named(_) => {
            let field_iter = stru.fields.iter().map(|field| {
                let ident = field.ident.as_ref().unwrap();
                quote!(#ident : self.#ident.lerp(&other.#ident, factor))
            });
            quote!(#stru_ident{#(#field_iter),*})
        }
        syn::Fields::Unnamed(_) => {
            let field_iter = stru
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| quote!(self.#i.lerp(&other.#i, factor)));
            quote!(#stru_ident(#(#field_iter),*))
        }
        syn::Fields::Unit => {
            quote!(#stru_ident)
        }
    };

    quote!(
        impl Lerp for #stru_ident {
            fn lerp(&self, other: &Self, factor: f32) -> Self {
                #lerp_impl_body
            }
        }
    )
    .into()
}
