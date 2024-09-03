mod container;
mod dynamo;
mod item;
mod table;
mod util;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Table, attributes(aws_dynamo))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    table::expand_table(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Item, attributes(aws_dynamo))]
pub fn derive_item(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    item::expand_item(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
