use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::key_schema::KeySchemaType;
use crate::util::to_pascal_case;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::BTreeMap;
use syn::Type;

#[derive(Clone, Debug)]
pub struct Container<'a> {
    /// field of struct
    pub field_ident: &'a Ident,
    /// type of field
    pub ty: &'a Type,
    /// key schemas parsed from attribute
    pub key_schemas: Vec<KeySchemaType>,
    /// ScalarAttributeTypes parsed from attribute
    pub attribute_definitions: Vec<ScalarAttributeType>,
    /// gsi index (index_name, KeySchemaType)
    pub global_secondary_index_key_schemas: BTreeMap<String, Vec<KeySchemaType>>,
    /// placeholder for conversions
    pub to_attribute_target_ident: &'a TokenStream,
    /// from Rust type to AttributeValueType
    pub to_attribute_token_stream: TokenStream,
    /// from AttributeValueType to Rust type
    pub from_attribute_token_stream: TokenStream,
}

impl<'a> Container<'a> {
    pub fn new(ident: &'a Ident, ty: &'a Type, to_attribute_target_ident: &'a TokenStream) -> Self {
        Self {
            field_ident: ident,
            ty,
            key_schemas: vec![],
            attribute_definitions: vec![],
            global_secondary_index_key_schemas: BTreeMap::new(),
            to_attribute_target_ident,
            to_attribute_token_stream: TokenStream::new(),
            from_attribute_token_stream: TokenStream::new(),
        }
    }
}

pub fn expand_impl_conversions(
    ident: &Ident,
    containers: &[Container],
) -> syn::Result<Vec<TokenStream>> {
    let mut impls = vec![];

    let map_inserts = containers
        .iter()
        .map(|c| {
            let ident_key = to_pascal_case(&c.field_ident.to_string());
            let to_attribute_token = &c.to_attribute_token_stream;
            quote! {
                map.insert(#ident_key.to_string(), #to_attribute_token);
            }
        })
        .collect::<Vec<_>>();

    let from_attr_fields = containers
        .iter()
        .map(|c| {
            let field_ident = c.field_ident;
            let from_attribute_token = &c.from_attribute_token_stream;
            quote! {
                #field_ident: #from_attribute_token
            }
        })
        .collect::<Vec<_>>();

    impls.push(quote! {
        impl From<#ident> for ::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue> {
            fn from(value: #ident) -> Self {
                (&value).into()
            }
        }
    });

    impls.push(quote! {
        impl From<&#ident> for ::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue> {
            fn from(value: &#ident) -> Self {
                let mut map = ::std::collections::HashMap::new();
                #( #map_inserts )*
                map
            }
        }
    });

    impls.push(quote! {
        impl TryFrom<::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue>>
        for #ident {
            type Error = ::aws_sdk_dynamodb::types::AttributeValue;
            fn try_from(value: ::std::collections::HashMap<
                ::std::string::String,
                ::aws_sdk_dynamodb::types::AttributeValue>
            ) -> Result<Self, Self::Error> {
                (&value).try_into()
            }
        }
    });

    impls.push(quote! {
        impl TryFrom<&::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue>>
        for #ident {
            type Error = ::aws_sdk_dynamodb::types::AttributeValue;
            fn try_from(value: &::std::collections::HashMap<
                ::std::string::String,
                ::aws_sdk_dynamodb::types::AttributeValue>
            ) -> Result<Self, Self::Error> {
                Ok(Self { #(# from_attr_fields ), * })
            }
        }
    });

    Ok(impls)
}
