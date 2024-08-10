use crate::ast::ext_field::ExtField;
use crate::ast::literal_input::LiteralInput;
use crate::dynamo::attribute_definition::get_attribute_definition;
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::dynamo::key_scheme::{get_key_scheme, Type as KeyType};
use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, Attribute, Data, DataStruct, DeriveInput, Error, Result};

const KEY_TABLE_NAME: &str = "table_name";

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
    let mut table_name = Literal::string(&to_pascal_case(&id.to_string()));

    for attr in attrs {
        attr.parse_nested_meta(|meta| {
            let key = meta
                .path
                .get_ident()
                .ok_or(Error::new(meta.path.span(), "key ident not found"))?;
            let value = meta.input.parse::<LiteralInput>()?.lit;
            if key.to_string().eq(KEY_TABLE_NAME) {
                table_name = value;
            }
            Ok(())
        })
        .ok();
    }
    Ok(quote! { .table_name(#table_name) })
}

fn extend_keys(ds: &DataStruct) -> Result<TokenStream> {
    let mut attribute_definitions = TokenStream::new();
    let mut key_schemas = TokenStream::new();

    for key_type in &[KeyType::HashKey, KeyType::RangeKey] {
        let keys = ExtField::get_paren_kv(&ds.fields, &key_type.to_string());
        for k in keys {
            let definition_type = parse2::<Literal>(k.ext)?;
            let id = k
                .ident
                .as_ref()
                .ok_or(Error::new(k.ident.span(), "invalid format"))?;

            let attribute_definition = get_attribute_definition(id, &definition_type)?;
            let key_schema = get_key_scheme(id, *key_type);

            attribute_definitions.extend(quote! { .attribute_definitions(#attribute_definition) });
            key_schemas.extend(quote! { .key_schema(#key_schema) });
        }
    }

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
