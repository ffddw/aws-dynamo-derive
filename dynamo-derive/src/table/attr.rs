use super::TABLE_ATTR_META_ENTRY;
use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::key_schema::KeySchemaType;
use crate::util::{strip_quote_mark, to_pascal_case};

use proc_macro2::Ident;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{parenthesized, Error, Field, Fields, LitStr, Result};

const GLOBAL_SECONDARY_INDEX_ENTRY: &str = "global_secondary_index";

#[derive(Debug)]
pub struct Attrs {
    pub hash_key: Ident,
    pub range_key: Option<Ident>,
    pub attribute_definitions: Vec<(Ident, ScalarAttributeType)>,
    pub global_secondary_indexes: Vec<(Ident, KeySchemaType)>,
}

impl Attrs {
    pub fn parse_table_fields(fields: &Fields) -> Result<Self> {
        let mut key_schemas = vec![];
        let mut attribute_definitions = vec![];
        let mut global_secondary_indexes = vec![];

        for field in fields {
            for attr in &field.attrs {
                if attr.path().is_ident(TABLE_ATTR_META_ENTRY) {
                    attr.parse_nested_meta(|table_meta| {
                        parse_key_schemas(
                            field,
                            &table_meta,
                            &mut key_schemas,
                            &mut attribute_definitions,
                        )?;
                        parse_global_secondary_index_key_schemas(
                            field,
                            &table_meta,
                            &mut global_secondary_indexes,
                            &mut attribute_definitions,
                        )?;
                        Ok(())
                    })?;
                }
            }
        }

        match key_schemas
            .iter()
            .filter(|(_, ks)| ks.eq(&KeySchemaType::HashKey))
            .count()
        {
            0 => Err(Error::new(fields.span(), "HashKey not found")),
            2.. => Err(Error::new(fields.span(), "only one HashKey is allowed")),
            1 => Ok(()),
        }?;

        if key_schemas
            .iter()
            .filter(|(_, ks)| ks.eq(&KeySchemaType::RangeKey))
            .count()
            > 1
        {
            return Err(Error::new(fields.span(), "at most one RangeKey is allowed"));
        };

        let hash_key = key_schemas
            .iter()
            .find(|(_, ks)| ks.eq(&KeySchemaType::HashKey))
            .map(|(ident, _)| ident.clone())
            .unwrap();

        let range_key = key_schemas
            .iter()
            .find(|(_, ks)| ks.eq(&KeySchemaType::RangeKey))
            .map(|(id, _)| id.clone());

        Ok(Self {
            hash_key,
            range_key,
            attribute_definitions,
            global_secondary_indexes,
        })
    }
}

fn parse_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    key_schemas: &mut Vec<(Ident, KeySchemaType)>,
    attribute_definitions: &mut Vec<(Ident, ScalarAttributeType)>,
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

            let ident = field
                .ident
                .as_ref()
                .ok_or(Error::new(field.ident.span(), "ident not found"))?;

            let pascal_cased_ident = Ident::new(&to_pascal_case(&ident.to_string()), ident.span());
            key_schemas.push((pascal_cased_ident.clone(), key_type));
            if !attribute_definitions.contains(&(pascal_cased_ident.clone(), scalar_attribute_type))
            {
                attribute_definitions.push((pascal_cased_ident, scalar_attribute_type));
            }
        }
    }
    Ok(())
}

fn parse_global_secondary_index_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    global_secondary_indexes: &mut Vec<(Ident, KeySchemaType)>,
    attribute_definitions: &mut Vec<(Ident, ScalarAttributeType)>,
) -> Result<()> {
    if table.path.is_ident(GLOBAL_SECONDARY_INDEX_ENTRY) {
        table.parse_nested_meta(|gsi| {
            parse_key_schemas(field, &gsi, global_secondary_indexes, attribute_definitions)?;
            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::dynamo::attribute_definition::ScalarAttributeType;
    use crate::table::attr::Attrs;

    use syn::{parse_quote, Fields, FieldsNamed, Result};

    #[test]
    fn test_invalid_key_attrs() -> Result<()> {
        let fields_named: FieldsNamed = parse_quote! {
            {
                #[table(hash_key("S"))]
                hk: String,
                #[table(hash_key("S"))]
                hk2: String
            }
        };
        let fields = Fields::Named(fields_named);
        assert_eq!(
            Attrs::parse_table_fields(&fields)
                .err()
                .unwrap()
                .to_string(),
            "only one HashKey is allowed"
        );

        let fields_named: FieldsNamed = parse_quote! {
            {
                #[table(hash_key("S"))]
                hk: String,
                #[table(range_key("N"))]
                rk: String,
                #[table(range_key("N"))]
                rk2: String
            }
        };
        let fields = Fields::Named(fields_named);
        assert_eq!(
            Attrs::parse_table_fields(&fields)
                .err()
                .unwrap()
                .to_string(),
            "at most one RangeKey is allowed"
        );
        Ok(())
    }

    #[test]
    fn test_valid_key_attrs() -> Result<()> {
        let fields_named: FieldsNamed = parse_quote! {
            {
                #[table(hash_key("S"))]
                hk: String,
                #[table(range_key("N"))]
                rk: String,
                #[table(global_secondary_index(range_key("S")))]
                gsi_hk: String
            }
        };
        let fields = Fields::Named(fields_named);
        let attr = Attrs::parse_table_fields(&fields)?;

        assert_eq!(attr.hash_key.to_string(), "Hk");
        assert_eq!(attr.range_key.as_ref().unwrap().to_string(), "Rk");
        assert_eq!(
            attr.attribute_definitions,
            vec![
                (attr.hash_key, ScalarAttributeType::S),
                (attr.range_key.unwrap().clone(), ScalarAttributeType::N),
                (
                    attr.global_secondary_indexes.first().unwrap().clone().0,
                    ScalarAttributeType::S
                )
            ]
        );

        Ok(())
    }
}
