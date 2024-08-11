mod attr;

use crate::dynamo::attribute_definition::expand_attribute_definition;
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::dynamo::key_scheme::{expand_key_schema, KeySchemaType};
use crate::table::attr::Attr;
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

    let keys = extend_keys(ds)?;
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

fn extend_keys(ds: &DataStruct) -> Result<TokenStream> {
    let attr = Attr::parse_table_fields(&ds.fields)?;

    let mut attribute_definitions = TokenStream::new();
    let mut key_schemas = TokenStream::new();

    attr.attribute_definitions
        .into_iter()
        .for_each(|(id, attr_ty)| {
            let attribute_definition = expand_attribute_definition(&id, attr_ty);
            attribute_definitions.extend(quote! { .attribute_definitions(#attribute_definition) })
        });

    let hash_key_tokens = expand_key_schema(&attr.hash_key, KeySchemaType::HashKey);
    key_schemas.extend(quote! { .key_schema(#hash_key_tokens) });

    attr.range_key.iter().for_each(|range_key| {
        let range_key_tokens = expand_key_schema(range_key, KeySchemaType::RangeKey);
        key_schemas.extend(quote! { .key_schema(#range_key_tokens) })
    });

    Ok(quote! {
        #attribute_definitions
        #key_schemas
    })
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
