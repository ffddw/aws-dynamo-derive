use crate::util::{strip_quote_mark, to_pascal_case};

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::{Error, Result};

pub fn get_attribute_definition(id: &Ident, attr_type: &Literal) -> Result<TokenStream> {
    let ident = Literal::string(&to_pascal_case(&id.to_string()));

    let scalar_attr_type = match strip_quote_mark(&attr_type.to_string())
        .ok_or(Error::new(id.span(), "invalid attribute format"))?
    {
        "S" => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::S },
        "N" => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::N },
        "B" => quote! { aws_sdk_dynamodb::types::ScalarAttributeType::B },
        _ => return Err(Error::new(attr_type.span(), "invalid attribute type")),
    };

    Ok(quote! {
        aws_sdk_dynamodb::types::AttributeDefinition::builder()
            .attribute_name(#ident.to_string())
            .attribute_type(#scalar_attr_type)
            .build()
            .unwrap()
    })
}
