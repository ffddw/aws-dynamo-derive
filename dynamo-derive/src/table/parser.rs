use super::TABLE_ATTR_META_ENTRY;
use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::attribute_value::AttributeValueType;
use crate::dynamo::key_schema::KeySchemaType;
use crate::util::strip_quote_mark;

use proc_macro2::Literal;
use std::collections::BTreeMap;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{parenthesized, Attribute, Error, Field, LitStr, Result};

const GLOBAL_SECONDARY_INDEX_ENTRY: &str = "global_secondary_index";
const GLOBAL_SECONDARY_INDEX_NAME: &str = "index_name";

pub fn parse_keys(
    attrs: &[Attribute],
    field: &Field,
    attribute_value_type: AttributeValueType,
    key_schemas: &mut Vec<KeySchemaType>,
    attribute_definitions: &mut Vec<ScalarAttributeType>,
    global_secondary_indexes: &mut BTreeMap<String, Vec<KeySchemaType>>,
) -> Result<()> {
    for attr in attrs {
        if attr.path().is_ident(TABLE_ATTR_META_ENTRY) {
            attr.parse_nested_meta(|table_meta| {
                parse_key_schemas(
                    field,
                    &table_meta,
                    attribute_value_type,
                    key_schemas,
                    attribute_definitions,
                )?;
                parse_global_secondary_index_key_schemas(
                    field,
                    &table_meta,
                    attribute_value_type,
                    attribute_definitions,
                    global_secondary_indexes,
                )?;
                Ok(())
            })?;
        }
    }

    Ok(())
}

fn parse_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    attribute_value_type: AttributeValueType,
    key_schemas: &mut Vec<KeySchemaType>,
    attribute_definitions: &mut Vec<ScalarAttributeType>,
) -> Result<()> {
    for key_type in [KeySchemaType::HashKey, KeySchemaType::RangeKey] {
        if table.path.is_ident(&key_type.to_string()) {
            let content;
            parenthesized!(content in table.input);
            let scalar_attribute_type: Option<LitStr> = content.parse().ok();

            let scalar_attribute_type = match strip_quote_mark(
                &scalar_attribute_type
                    .clone()
                    .ok_or(Error::new(field.span(), "invalid key type format"))?
                    .token()
                    .to_string(),
            )
            .unwrap()
            {
                "B" => ScalarAttributeType::B,
                "N" => ScalarAttributeType::N,
                "S" => ScalarAttributeType::S,
                _ => {
                    return Err(Error::new(
                        scalar_attribute_type.span(),
                        "invalid ScalarAttributeType",
                    ))
                }
            };

            scalar_attribute_type.validate_type(&field.ty, attribute_value_type)?;

            key_schemas.push(key_type);
            if !attribute_definitions.contains(&scalar_attribute_type) {
                attribute_definitions.push(scalar_attribute_type);
            }
        }
    }
    Ok(())
}

fn parse_global_secondary_index_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    attribute_value_type: AttributeValueType,
    attribute_definitions: &mut Vec<ScalarAttributeType>,
    global_secondary_indexes: &mut BTreeMap<String, Vec<KeySchemaType>>,
) -> Result<()> {
    if table.path.is_ident(GLOBAL_SECONDARY_INDEX_ENTRY) {
        let mut index_name = String::from("");
        table.parse_nested_meta(|gsi| {
            let mut key_schemas = vec![];
            if gsi.path.is_ident(GLOBAL_SECONDARY_INDEX_NAME) {
                index_name = strip_quote_mark(&gsi.value()?.parse::<Literal>()?.to_string())
                    .ok_or(gsi.error("invalid index name"))?
                    .to_string();
            } else {
                parse_key_schemas(
                    field,
                    &gsi,
                    attribute_value_type,
                    &mut key_schemas,
                    attribute_definitions,
                )?;
            }

            if index_name.is_empty() {
                return Err(gsi.error("empty index name"));
            };

            global_secondary_indexes
                .entry(index_name.clone())
                .or_default()
                .extend(key_schemas);
            Ok(())
        })?;
    }

    Ok(())
}
