mod attr;

use crate::dynamo::attribute_definition::expand_attribute_definition;
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::dynamo::key_schema::expand_key_schema;
use crate::table::attr::Attrs;
use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Error, LitStr, Result};

const KEY_TABLE_NAME: &str = "table_name";
const TABLE_ATTR_META_ENTRY: &str = "table";

pub fn expand_table(input: &mut DeriveInput) -> Result<TokenStream> {
    let table_name = extend_table_name(&input.ident, &input.attrs)?;
    let ds = match &input.data {
        Data::Struct(ds) => ds,
        _ => return Err(Error::new(input.span(), "only struct type available")),
    };

    let mut attrs = Attrs::parse_table_fields(&ds.fields)?;
    let keys = extend_keys(&attrs);
    let gsi_key_schemas = extend_global_secondary_index_key_schemas(&mut attrs);
    let items = extend_items(ds)?;
    let generics = &input.generics;
    let vis = &input.vis;
    let struct_name = &input.ident;

    let mut out = TokenStream::new();

    out.extend(quote! {
        impl #generics #struct_name #generics {
            #vis fn create_table(mut builder: aws_sdk_dynamodb::operation::create_table::builders::CreateTableFluentBuilder)
            -> aws_sdk_dynamodb::operation::create_table::builders::CreateTableFluentBuilder {
                builder
                    #table_name
                    #keys
            }

            #vis fn get_global_secondary_index_key_schemas() -> std::collections::BTreeMap<String, Vec<aws_sdk_dynamodb::types::KeySchemaElement>> {
                #gsi_key_schemas
            }

            #vis fn put_item(&self, mut builder: aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder)
            -> aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder {
                builder
                    #table_name
                    #items
            }
        }
    });

    Ok(out)
}

fn extend_table_name(id: &Ident, attrs: &[Attribute]) -> Result<TokenStream> {
    let mut table_name = LitStr::new(&to_pascal_case(&id.to_string()), id.span());

    for attr in attrs {
        if attr.path().is_ident(TABLE_ATTR_META_ENTRY) {
            attr.parse_nested_meta(|table| {
                if table.path.is_ident(KEY_TABLE_NAME) {
                    table_name = table.value()?.parse()?;
                }
                Ok(())
            })?;
        }
    }
    Ok(quote! { .table_name(#table_name) })
}

fn extend_keys(attrs: &Attrs) -> TokenStream {
    let mut attribute_definitions = TokenStream::new();
    let mut key_schemas = TokenStream::new();

    attrs
        .attribute_definitions
        .iter()
        .for_each(|(id, attr_ty)| {
            let attribute_definition = expand_attribute_definition(id, attr_ty);
            attribute_definitions.extend(quote! { .attribute_definitions(#attribute_definition) })
        });

    attrs
        .key_schemas
        .iter()
        .for_each(|(ident, key_schema_type)| {
            let key_schema_token = expand_key_schema(ident, *key_schema_type);
            key_schemas.extend(quote! { .key_schema(#key_schema_token) })
        });

    quote! {
        #attribute_definitions
        #key_schemas
    }
}

fn extend_global_secondary_index_key_schemas(attrs: &mut Attrs) -> TokenStream {
    let mut gsi_key_schemas = quote! {
        let mut gsi_key_schemas: std::collections::BTreeMap<
            String,
            Vec<aws_sdk_dynamodb::types::KeySchemaElement>,
        > = std::collections::BTreeMap::new();
    };

    attrs
        .global_secondary_indexes
        .iter_mut()
        .for_each(|(index_name, items)| {
            items.sort_by_key(|(_, ty)| *ty);
            items.iter().for_each(|(ident, key_schema_type)| {
                let gsi_key_schemas_token = expand_key_schema(ident, *key_schema_type);
                gsi_key_schemas.extend(quote! {
                    gsi_key_schemas.entry(#index_name.to_string()).or_default().push(#gsi_key_schemas_token);
                });
            });
        });
    gsi_key_schemas.extend(quote! { gsi_key_schemas });

    gsi_key_schemas
}

fn extend_items(ds: &DataStruct) -> Result<TokenStream> {
    let mut items = TokenStream::new();

    for field in &ds.fields {
        let ty = &field.ty;
        let ident = field
            .ident
            .as_ref()
            .ok_or(Error::new(field.ident.span(), "field ident not found"))?;
        let ident_lit = Literal::string(&to_pascal_case(&ident.to_string()));
        let (item, _) = expand_attribute_value(ident, ty, 0)?.clone();

        items.extend(quote! {
            .item(#ident_lit.to_string(), #item)
        });
    }

    Ok(items)
}
