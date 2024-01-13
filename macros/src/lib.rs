use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote, TypePath};

///
/// A struct like this:
/// ```rust,no-run
/// pub struct RendererDependencies {
///     scheduler: Handle<Scheduler>,
///     graphics: Handle<GraphicsContext>,
/// }
/// ```
/// Will be expanded into this:
/// ```rust,no-run
/// impl Dependencies for RendererDependencies {
///     fn type_ids() -> Vec<crate::app::ModuleId> {
///         let mut ids = ::alloc::vec::Vec::new();
///         ids.extend(<Handle<Scheduler> as Dependencies>::type_ids());
///         ids.extend(<Handle<GraphicsContext> as Dependencies>::type_ids());
///         ids
///     }
///     fn from_untyped_handles(ptrs: &[crate::app::UntypedHandle]) -> Self {
///         let mut offset: usize = 0;
///         let ids = <Handle<Scheduler> as Dependencies>::type_ids();
///         let range_0 = offset..(offset + ids.len());
///         offset += ids.len();
///         let ids = <Handle<GraphicsContext> as Dependencies>::type_ids();
///         let range_1 = offset..(offset + ids.len());
///         offset += ids.len();
///         Self {
///             scheduler: <Handle<
///                 Scheduler,
///             > as Dependencies>::from_untyped_handles(&ptrs[range_0]),
///             graphics: <Handle<
///                 GraphicsContext,
///             > as Dependencies>::from_untyped_handles(&ptrs[range_1]),
///         }
///     }
/// }
/// ```   
#[proc_macro_derive(Dependencies)]
pub fn derive_dependencies(input: TokenStream) -> TokenStream {
    let derive_input: syn::DeriveInput = syn::parse(input).unwrap();
    let stru = match &derive_input.data {
        syn::Data::Struct(s) => s,
        _ => panic!("Only derive Dependencies on structs"),
    };
    let stru_ident = derive_input.ident;
    // let fields: Vec<TypePath>
    // s.fields.iter().map(|e|)

    let mut field_type_paths: Vec<&TypePath> = vec![];

    for e in stru.fields.iter() {
        let path = match &e.ty {
            syn::Type::Path(path) => path,
            _ => panic!(
                "field {:?} is not a Handle! ",
                e.to_token_stream().to_string()
            ),
        };
        field_type_paths.push(path);
    }

    let extend_ids = field_type_paths
        .iter()
        .map(|path| quote!(ids.extend(<#path as Dependencies>::type_ids());))
        .collect::<Vec<proc_macro2::TokenStream>>();

    let declare_element_ranges = field_type_paths
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let range_i: syn::Ident = syn::parse_str(&format!("range_{i}")).unwrap();

            let declare_offset_if_first = if i == 0 {
                Some(quote!(let mut offset: usize = 0;))
            } else {
                None
            };
            quote!(
                #declare_offset_if_first

                let ids = <#path as Dependencies>::type_ids();
                let #range_i = offset..(offset + ids.len());
                offset += ids.len();

            )
        })
        .collect::<Vec<proc_macro2::TokenStream>>();

    let self_construction_from_ranges = match &stru.fields {
        syn::Fields::Unit => quote!(Self),
        syn::Fields::Named(named_fields) => {
            assert_eq!(field_type_paths.len(), named_fields.named.len());
            let set_fields = named_fields
                .named
                .iter()
                .zip(field_type_paths.iter())
                .enumerate()
                .map(|(i, (field, path))| {
                    let range_i: syn::Ident = syn::parse_str(&format!("range_{i}")).unwrap();
                    let field_ident = field.ident.as_ref().unwrap();
                    quote!(#field_ident: <#path as Dependencies>::from_untyped_handles(&ptrs[#range_i]))
                });

            quote!(
                Self {
                    #( #set_fields ),*
                }
            )
        }
        syn::Fields::Unnamed(unnamed_fields) => {
            assert_eq!(field_type_paths.len(), unnamed_fields.unnamed.len());

            let set_fields = field_type_paths.iter().enumerate().map(|(i, path)| {
                let range_i: syn::Ident = syn::parse_str(&format!("range_{i}")).unwrap();
                quote!(<#path as Dependencies>::from_untyped_handles(&ptrs[#range_i]))
            });

            quote!(
                Self (
                    #( #set_fields ),*
                )
            )
        }
    };

    let trait_impl: proc_macro2::TokenStream = quote!(
        impl Dependencies for #stru_ident {
            fn type_ids() -> Vec<crate::app::ModuleId> {
                let mut ids = vec![];
                #( #extend_ids )*
                ids
            }

            fn from_untyped_handles(ptrs: &[crate::app::UntypedHandle]) -> Self {
                #( #declare_element_ranges)*

                #self_construction_from_ranges
            }
        }
    );

    trait_impl.into()
}

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
