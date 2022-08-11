#![doc(html_no_source)]

use proc_macro::TokenStream;
use quote::quote;

/// Create a read only register
#[proc_macro_derive(RORegister)]
pub fn ro_register(input: TokenStream) -> TokenStream {
    // Parse the representation
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let output = impl_ro_register(&ast);
    output.into()
}

fn impl_ro_register(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    quote! {
        #[allow(dead_code)]
        impl crate::registers::Register for #name {
            const ADDRESS: crate::address::Address = crate::address::Address::#name;
        }
    }
}

/// Create a read/write register
#[proc_macro_derive(RERegister)]
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
        impl crate::registers::EditableRegister for #name {}
    }
}

/// Create a read/write register
#[proc_macro_derive(RWRegister)]
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
        impl crate::registers::WritableRegister for #name {}
    }
}
