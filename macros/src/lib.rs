use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Struct, DeriveInput, ImplItem,
    ItemImpl, ItemStruct, ItemTrait, ItemType, TypeImplTrait,
};

fn trait_struct_ident(trait_ident: &Ident) -> Ident {
    syn::parse_str(&format!("Dyn{}", trait_ident.to_string())).unwrap()
}

#[proc_macro]
pub fn reflect(tokens: TokenStream) -> TokenStream {
    if let Ok(trait_ident) = syn::parse::<Ident>(tokens.clone()) {
        let trait_struct_ident = trait_struct_ident(&trait_ident);
        let code = quote! {
            pub struct #trait_struct_ident;
            impl ReflectedTrait for #trait_struct_ident {
                type Dyn = dyn #trait_ident;
            }
            impl ReflectedTraitInv for dyn #trait_ident {
                type Struct = #trait_struct_ident;
            }
        };
        return code.into();
    }

    let tokens_string = tokens.to_string();
    let split: Vec<&str> = tokens_string.split(",").map(|e| e.trim()).collect();
    if split.len() == 2 {
        let trait_ident: Ident =
            syn::parse_str(&split[0]).expect("first element after comma must be a trait ident");
        let struct_ident: Ident =
            syn::parse_str(&split[1]).expect("second element before comma must be a struct ident");
        let trait_struct_ident = trait_struct_ident(&trait_ident);

        let code = quote! {
            impl Implements<#trait_struct_ident> for #struct_ident {
                unsafe fn uninit_trait_obj() -> Option<&'static dyn #trait_ident> {
                    const UNINIT: #struct_ident =
                        unsafe { std::mem::MaybeUninit::<#struct_ident>::uninit().assume_init() };
                    Some(&UNINIT as &'static dyn #trait_ident)
                }
            }
        };
        return code.into();
    }

    panic!("Invalid");

    // let item: syn::Itent;
    // let item: syn::Item =
    //     syn::parse(item).expect("the `reflect` macro must be applied on a trait or struct");

    // match &item {
    //     syn::Item::Impl(e) => {
    //         let Some(tr) = &e.trait_ else {
    //             panic!("Impl must be for a trait")
    //         };
    //         todo!()
    //     }
    //     syn::Item::Trait(tr) => {
    //         let trait_ident = tr.ident.clone();
    //         let trait_struct_ident: Ident =
    //             syn::parse_str(&format!("Dyn{}", trait_ident.to_string())).unwrap();
    //         quote! {
    //             #item
    //             struct #trait_struct_ident;
    //             impl ReflectedTrait for #trait_struct_ident {
    //                 type Dyn = dyn #trait_ident;
    //             }
    //             impl ReflectedTraitInv for dyn #trait_ident {
    //                 type Struct = #trait_struct_ident;
    //             }
    //         }
    //         .into()
    //     }
    //     _ => panic!("Use the reflect macro only on traits or trait impls"),
    // }
}
