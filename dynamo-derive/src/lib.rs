mod ast;
mod dynamo;
mod table;
mod util;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Table, attributes(table))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    table::expand_table(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
