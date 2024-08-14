use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::attribute_value::AttributeValueType;
use crate::dynamo::key_schema::KeySchemaType;

use proc_macro2::{Ident, TokenStream};
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
    /// stack of AttributeValueType mapped to field type
    pub attr_value_types: Vec<AttributeValueType>,
    /// from Rust type to AttributeValueType
    pub to_attribute_token_stream: TokenStream,
    /// from AttributeValueType to Rust type
    pub from_attribute_token_stream: TokenStream,
}

impl<'a> Container<'a> {
    pub fn new(ident: &'a Ident, ty: &'a Type) -> Self {
        Self {
            field_ident: ident,
            ty,
            key_schemas: vec![],
            attribute_definitions: vec![],
            global_secondary_index_key_schemas: BTreeMap::new(),
            attr_value_types: vec![],
            to_attribute_token_stream: TokenStream::new(),
            from_attribute_token_stream: TokenStream::new(),
        }
    }
}
