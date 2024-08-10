use proc_macro2::Literal;
use syn::parse::{Parse, ParseStream};
use syn::Token;

#[derive(Debug)]
pub struct LiteralInput {
    pub _eq_token: Token![=],
    pub lit: Literal,
}

impl Parse for LiteralInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _eq_token: input.parse()?,
            lit: input.parse()?,
        })
    }
}
