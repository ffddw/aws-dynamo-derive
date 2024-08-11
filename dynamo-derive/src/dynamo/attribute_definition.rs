use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ScalarAttributeType {
    B,
    N,
    S,
}

pub fn expand_attribute_definition(id: &Ident, attr_type: &ScalarAttributeType) -> TokenStream {
    let ident = Literal::string(&to_pascal_case(&id.to_string()));

    let scalar_attr_type = match attr_type {
        ScalarAttributeType::S => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::S },
        ScalarAttributeType::N => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::N },
        ScalarAttributeType::B => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::B },
    };

    quote! {
        aws_sdk_dynamodb::types::AttributeDefinition::builder()
            .attribute_name(#ident.to_string())
            .attribute_type(#scalar_attr_type)
            .build()
            .unwrap()
    }
}
