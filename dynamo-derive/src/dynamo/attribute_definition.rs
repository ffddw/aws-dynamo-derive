use crate::dynamo::attribute_value::AttributeValueType;
use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use syn::{Error, Result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ScalarAttributeType {
    B,
    N,
    S,
}

impl ScalarAttributeType {
    pub fn from_attribute_value_type(
        attr_value_ty: AttributeValueType,
        span: Span,
    ) -> Result<Self> {
        let scalar_attr_type = match attr_value_ty {
            AttributeValueType::B => Self::B,
            AttributeValueType::S => Self::S,
            AttributeValueType::N => Self::N,
            _ => {
                return Err(Error::new(
                    span,
                    format!("invalid type for ScalarAttributeType: {:?}", attr_value_ty),
                ))
            }
        };
        Ok(scalar_attr_type)
    }
}

pub fn expand_attribute_definition(id: &Ident, attr_type: &ScalarAttributeType) -> TokenStream {
    let ident = Literal::string(&to_pascal_case(&id.to_string()));

    let scalar_attr_type = match attr_type {
        ScalarAttributeType::S => quote! { ::aws_sdk_dynamodb::types::ScalarAttributeType::S },
        ScalarAttributeType::N => quote! { ::aws_sdk_dynamodb::types::ScalarAttributeType::N },
        ScalarAttributeType::B => quote! { ::aws_sdk_dynamodb::types::ScalarAttributeType::B },
    };

    quote! {
        aws_sdk_dynamodb::types::AttributeDefinition::builder()
            .attribute_name(#ident)
            .attribute_type(#scalar_attr_type)
            .build()
            .unwrap()
    }
}
