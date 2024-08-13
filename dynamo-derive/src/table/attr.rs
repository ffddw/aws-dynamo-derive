use super::TABLE_ATTR_META_ENTRY;
use crate::dynamo::attribute_definition::ScalarAttributeType;
use crate::dynamo::key_schema::KeySchemaType;
use crate::util::{strip_quote_mark, to_pascal_case};

use proc_macro2::{Ident, Literal, Span};
use std::collections::BTreeMap;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{parenthesized, Error, Field, Fields, LitStr, Result};

const GLOBAL_SECONDARY_INDEX_ENTRY: &str = "global_secondary_index";
const GLOBAL_SECONDARY_INDEX_NAME: &str = "index_name";

pub struct Attrs {
    pub key_schemas: Vec<(Ident, KeySchemaType)>,
    pub attribute_definitions: Vec<(Ident, ScalarAttributeType)>,
    pub global_secondary_indexes: BTreeMap<String, Vec<(Ident, KeySchemaType)>>,
}

impl Attrs {
    pub fn parse_table_fields(fields: &Fields) -> Result<Self> {
        let mut key_schemas = vec![];
        let mut attribute_definitions = vec![];
        let mut global_secondary_indexes = BTreeMap::new();

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

        validate_and_sort_key_schemas(&mut key_schemas, fields.span())?;
        global_secondary_indexes
            .iter_mut()
            .try_for_each(|(_, gsi_key_schemas)| {
                validate_and_sort_key_schemas(gsi_key_schemas, fields.span())
            })?;

        Ok(Self {
            key_schemas,
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

            scalar_attribute_type.validate_type(&field.ty)?;

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

fn validate_and_sort_key_schemas(
    key_schemas: &mut [(Ident, KeySchemaType)],
    span: Span,
) -> Result<()> {
    match key_schemas
        .iter()
        .filter(|(_, ks)| ks.eq(&KeySchemaType::HashKey))
        .count()
    {
        0 => Err(Error::new(span, "HashKey not found")),
        2.. => Err(Error::new(span, "only one HashKey is allowed")),
        1 => Ok(()),
    }?;

    if key_schemas
        .iter()
        .filter(|(_, ks)| ks.eq(&KeySchemaType::RangeKey))
        .count()
        > 1
    {
        return Err(Error::new(span, "at most one RangeKey is allowed"));
    };

    key_schemas.sort_by_key(|(_, k)| *k);

    Ok(())
}

fn parse_global_secondary_index_key_schemas(
    field: &Field,
    table: &ParseNestedMeta,
    global_secondary_indexes: &mut BTreeMap<String, Vec<(Ident, KeySchemaType)>>,
    attribute_definitions: &mut Vec<(Ident, ScalarAttributeType)>,
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
                parse_key_schemas(field, &gsi, &mut key_schemas, attribute_definitions)?;
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

#[cfg(test)]
mod test {
    use crate::dynamo::attribute_definition::ScalarAttributeType;
    use crate::dynamo::key_schema::KeySchemaType;
    use crate::table::attr::Attrs;

    use proc_macro2::Literal;
    use syn::{parenthesized, parse_quote, Attribute, Fields, FieldsNamed, Result};

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
                rk: i32,
                #[table(range_key("N"))]
                rk2: u128
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
                rk: u32,
                #[table(global_secondary_index(index_name="test_idx", hash_key("S")))]
                gsi_hk: String,
                #[table(global_secondary_index(index_name="test_idx2", hash_key("S")))]
                gsi_hk2: String
            }
        };
        let fields = Fields::Named(fields_named);
        let attr = Attrs::parse_table_fields(&fields)?;

        let hk = attr
            .key_schemas
            .iter()
            .find(|(_, k)| k.eq(&KeySchemaType::HashKey))
            .unwrap();
        let rk = attr
            .key_schemas
            .iter()
            .find(|(_, k)| k.eq(&KeySchemaType::RangeKey))
            .unwrap();

        assert_eq!(hk.0.to_string(), "Hk");
        assert_eq!(rk.0, "Rk");
        assert_eq!(
            attr.attribute_definitions,
            vec![
                (hk.0.clone(), ScalarAttributeType::S),
                (rk.0.clone(), ScalarAttributeType::N),
                (
                    attr.global_secondary_indexes
                        .get("test_idx")
                        .unwrap()
                        .first()
                        .unwrap()
                        .0
                        .clone(),
                    ScalarAttributeType::S
                ),
                (
                    attr.global_secondary_indexes
                        .get("test_idx2")
                        .unwrap()
                        .first()
                        .unwrap()
                        .0
                        .clone(),
                    ScalarAttributeType::S
                ),
            ]
        );
        assert_eq!(
            attr.global_secondary_indexes.get("test_idx").unwrap().len(),
            1
        );
        assert_eq!(
            attr.global_secondary_indexes
                .get("test_idx2")
                .unwrap()
                .len(),
            1
        );

        Ok(())
    }

    #[test]
    fn test() -> Result<()> {
        let attr: Attribute = parse_quote! {
            #[table(global_secondary_index(index_name="test_idx", range_key("S")))]
        };

        if attr.path().is_ident("table") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("global_secondary_index") {
                    meta.parse_nested_meta(|meta| {
                        if meta.path.is_ident("index_name") {
                            meta.value()?.parse::<Literal>()?;
                            Ok(())
                        } else if meta.path.is_ident("range_key") {
                            let content;
                            parenthesized!(content in meta.input);
                            content.parse::<Option<Literal>>().ok();
                            Ok(())
                        } else {
                            Err(meta.error("unsupported ingredient"))
                        }
                    })
                } else {
                    Err(meta.error("unsupported tea property"))
                }
            })?;
        }

        Ok(())
    }
}
