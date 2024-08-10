use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone)]
pub enum Type {
    HashKey,
    RangeKey,
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::HashKey => "hash_key",
            Self::RangeKey => "range_key",
        };
        write!(f, "{val}")
    }
}

pub fn get_key_scheme(id: &Ident, key_type: Type) -> TokenStream {
    let ident = Literal::string(&to_pascal_case(&id.to_string()));

    let key_type = match key_type {
        Type::HashKey => quote! { aws_sdk_dynamodb::types::KeyType::Hash },
        Type::RangeKey => quote! { aws_sdk_dynamodb::types::KeyType::Range },
    };

    quote! {
        aws_sdk_dynamodb::types::KeySchemaElement::builder()
            .attribute_name(#ident.to_string())
            .key_type(#key_type)
            .build()
            .unwrap()
    }
}
