#![doc(html_no_source)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitInt};

// struct Address(u32);
// impl syn::parse::Parse for Address {
//     fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//         let content;
//         syn::parenthesized!(content in input);
//         let address: u32 = content.parse()?;
//         Ok(Self(address))
//     }
// }

/// Create a read only register
#[proc_macro_derive(RORegister, attributes(address))]
pub fn ro_register(input: TokenStream) -> TokenStream {
    // Parse the representation
    // let args = parse_macro_input!(input as AttributeArgs);
    let ast: DeriveInput = syn::parse(input).unwrap();

    // Build the impl
    let output = impl_ro_register(&ast);
    output.into()
}

fn impl_ro_register(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let mut address: Option<u8> = None;

    for attr in ast.clone().attrs {
        if attr.path.is_ident("address") {
            let addr: LitInt = attr
                .parse_args()
                .expect("The `address` attribute is required");
            let num: u8 = addr
                .base10_parse()
                .expect("The `address` attribute must have a u8 as argument");

            if address.is_some() {
                panic!("Multiple `address` defined")
            }
            address = Some(num);
        }
    }

    let addr: u8 = address.expect("The `address` attribute is not defined");
    quote! {
        #[allow(dead_code)]
        impl crate::register::Register for #name {
            const ADDRESS: u8 = #addr;
        }
    }
}

/// Create a read/write register
#[proc_macro_derive(RERegister, attributes(address))]
pub fn re_register(input: TokenStream) -> TokenStream {
    // Parse the representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let read = impl_ro_register(&ast);
    let edit = impl_re_register(&ast);

    let read_edit = quote! {
        #read
        #edit
    };
    read_edit.into()
}
fn impl_re_register(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    quote! {
        #[allow(dead_code)]
        impl crate::register::EditableRegister for #name {}
    }
}

/// Create a read/write register
#[proc_macro_derive(RWRegister, attributes(address))]
pub fn rw_register(input: TokenStream) -> TokenStream {
    // Parse the representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let read = impl_ro_register(&ast);
    let edit = impl_re_register(&ast);
    let write = impl_rw_register(&ast);

    let read_write = quote! {
        #read
        #edit
        #write
    };
    read_write.into()
}
fn impl_rw_register(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    quote! {
        #[allow(dead_code)]
        impl crate::register::WritableRegister for #name {}
    }
}
