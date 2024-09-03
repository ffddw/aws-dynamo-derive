use crate::dynamo::attribute_value::AttributeValueType;
use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{Error, Result};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ScalarAttributeType {
    B,
    N,
    S,
}

impl ToTokens for ScalarAttributeType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(format_ident!("{}", format!("{:?}", self)))
    }
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

    pub fn expand_attribute_definition(&self, ident: &Ident) -> TokenStream {
        let ident = Literal::string(&to_pascal_case(&ident.to_string()));
        quote! {
            aws_sdk_dynamodb::types::AttributeDefinition::builder()
            .attribute_name(#ident)
            .attribute_type(::aws_sdk_dynamodb::types::ScalarAttributeType::#self)
            .build()
            .unwrap()
        }
    }
}
