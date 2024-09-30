use crate::container::Container;
use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::attribute_value::AttributeValueType;
use crate::dynamo::key_schema::KeySchemaType;
use crate::table::tags::AWS_DYNAMO_ATTR_META_ENTRY;
use crate::util::strip_quote_mark;

use proc_macro2::Literal;
use std::collections::BTreeMap;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Field, Result};

const LOCAL_SECONDARY_INDEX_ENTRY: &str = "local_secondary_index";
const GLOBAL_SECONDARY_INDEX_ENTRY: &str = "global_secondary_index";
const SECONDARY_INDEX_NAME: &str = "index_name";

pub fn parse_from_dynamo_attrs(
    attrs: &[Attribute],
    field: &Field,
    attribute_value_type: AttributeValueType,
    container: &mut Container,
) -> Result<()> {
    for attr in attrs {
        if attr.path().is_ident(AWS_DYNAMO_ATTR_META_ENTRY) {
            attr.parse_nested_meta(|table_meta| {
                parse_key_schemas(
                    &[KeySchemaType::HashKey, KeySchemaType::RangeKey],
                    field,
                    &table_meta,
                    attribute_value_type,
                    &mut container.key_schemas,
                    &mut container.attribute_definitions,
                )?;
                parse_local_secondary_index_key_schemas(
                    field,
                    &table_meta,
                    attribute_value_type,
                    &mut container.attribute_definitions,
                    &mut container.local_secondary_index_key_schemas,
                )?;
                parse_global_secondary_index_key_schemas(
                    field,
                    &table_meta,
                    attribute_value_type,
                    &mut container.attribute_definitions,
                    &mut container.global_secondary_index_key_schemas,
                )?;

                Ok(())
            })?;
        }
    }

    Ok(())
}

fn parse_key_schemas(
    key_types: &[KeySchemaType],
    field: &Field,
    table: &ParseNestedMeta,
    attribute_value_type: AttributeValueType,
    key_schemas: &mut Vec<KeySchemaType>,
    attribute_definitions: &mut Vec<ScalarAttributeType>,
) -> Result<()> {
    for key_type in key_types {
        if table.path.is_ident(&key_type.to_string()) {
            let scalar_attribute_type = ScalarAttributeType::from_attribute_value_type(
                attribute_value_type,
                field.ty.span(),
            )?;

            key_schemas.push(*key_type);

            if !attribute_definitions.contains(&scalar_attribute_type) {
                attribute_definitions.push(scalar_attribute_type);
            }
        }
    }
    Ok(())
}

fn parse_local_secondary_index_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    attribute_value_type: AttributeValueType,
    attribute_definitions: &mut Vec<ScalarAttributeType>,
    local_secondary_indexes: &mut BTreeMap<String, Vec<KeySchemaType>>,
) -> Result<()> {
    if table.path.is_ident(LOCAL_SECONDARY_INDEX_ENTRY) {
        let mut index_name = String::from("");

        table.parse_nested_meta(|nested_meta| {
            let mut key_schemas = vec![];

            if nested_meta.path.is_ident(SECONDARY_INDEX_NAME) {
                index_name =
                    strip_quote_mark(&nested_meta.value()?.parse::<Literal>()?.to_string())
                        .ok_or(nested_meta.error("invalid index name"))?
                        .to_string();
            } else {
                parse_key_schemas(
                    &[KeySchemaType::HashKey, KeySchemaType::RangeKey],
                    field,
                    &nested_meta,
                    attribute_value_type,
                    &mut key_schemas,
                    attribute_definitions,
                )?;
            }

            if index_name.is_empty() {
                return Err(nested_meta.error("empty index name"));
            };

            local_secondary_indexes
                .entry(index_name.clone())
                .or_default()
                .extend(key_schemas);
            Ok(())
        })?;
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

        table.parse_nested_meta(|nested_meta| {
            let mut key_schemas = vec![];

            if nested_meta.path.is_ident(SECONDARY_INDEX_NAME) {
                index_name =
                    strip_quote_mark(&nested_meta.value()?.parse::<Literal>()?.to_string())
                        .ok_or(nested_meta.error("invalid index name"))?
                        .to_string();
            } else {
                parse_key_schemas(
                    &[KeySchemaType::HashKey, KeySchemaType::RangeKey],
                    field,
                    &nested_meta,
                    attribute_value_type,
                    &mut key_schemas,
                    attribute_definitions,
                )?;
            }

            if index_name.is_empty() {
                return Err(nested_meta.error("empty index name"));
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
