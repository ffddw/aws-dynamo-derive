mod parser;
mod types;

use crate::case::Case;
use crate::container;
use crate::container::Container;
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::dynamo::key_schema::{expand_key_schema, validate_and_sort_key_schemas, KeySchemaType};
use crate::table::parser::parse_from_dynamo_attrs;
use crate::table::types::TableName;
use crate::tags::{
    AWS_DYNAMO_ATTR_META_ENTRY, KEY_RENAME, KEY_TABLE_NAME, KEY_TABLE_NAME_FN,
    PRIMARY_KEY_INPUT_STRUCT_POSTFIX,
};
use crate::util::to_snake_case;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Error, LitStr, Result, Visibility};

struct TableMeta {
    table_name: TableName,
    case: Case,
}

pub fn expand_table(input: &mut DeriveInput) -> Result<TokenStream> {
    let input_span = input.span();

    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = input;

    let TableMeta { table_name, case } = get_table_meta(ident, attrs)?;
    let ds = match &data {
        Data::Struct(ds) => ds,
        _ => return Err(Error::new(input.span(), "only struct type available")),
    };

    let to_attribute_ident = quote! { self };
    let from_attribute_ident = quote! { value };
    let attribute_types_containers =
        get_attribute_types_containers(ds, &to_attribute_ident, &from_attribute_ident, case)?;

    let prelude_structs = expand_prelude_structs(vis, ident, &attribute_types_containers);

    // expands functions
    let (
        get_table_name_fn,
        create_table_fn,
        local_secondary_index_key_schemas_fn,
        global_secondary_index_key_schemas_fn,
        from_attribute_value_fn,
        put_item_fn,
        get_primary_keys_fn,
        conversions,
    ) = (
        expand_get_table_name_fn(&table_name),
        expand_create_table_fn(&attribute_types_containers, &table_name, case, input_span)?,
        expand_local_secondary_index_key_schemas_fn(&attribute_types_containers, case, input_span)?,
        expand_global_secondary_index_key_schemas_fn(
            &attribute_types_containers,
            case,
            input_span,
        )?,
        expand_from_attribute_value_fn(&attribute_types_containers, &from_attribute_ident),
        expand_put_item_fn(&attribute_types_containers, &table_name, case),
        expand_get_primary_keys_fn(ident, case, &attribute_types_containers)?,
        expand_impl_conversions(ident, ds, case)?,
    );

    Ok(quote! {
        #( #prelude_structs )*

        #(
            #[allow(clippy::needless_question_mark)]
            #conversions
        )*

        #[allow(clippy::map_clone)]
        #[allow(clippy::needless_question_mark)]
        #[allow(dead_code)]
        impl #generics #ident #generics {
            #vis #get_table_name_fn
            #vis #create_table_fn
            #vis #local_secondary_index_key_schemas_fn
            #vis #global_secondary_index_key_schemas_fn
            #vis #from_attribute_value_fn
            #vis #put_item_fn
            #vis #get_primary_keys_fn
        }
    })
}

fn get_table_meta(id: &Ident, attrs: &[Attribute]) -> Result<TableMeta> {
    let mut table_name = None;
    let mut case = Case::default();

    for attr in attrs {
        if attr.path().is_ident(AWS_DYNAMO_ATTR_META_ENTRY) {
            attr.parse_nested_meta(|table| {
                if table.path.is_ident(KEY_TABLE_NAME) {
                    if table_name.is_some() {
                        return Err(Error::new(
                            table.path.span(),
                            "table name already set. static str and function cannot be used both",
                        ));
                    }
                    table_name = Some(TableName::StaticStr(table.value()?.parse()?));
                }
                if table.path.is_ident(KEY_TABLE_NAME_FN) {
                    if table_name.is_some() {
                        return Err(Error::new(
                            table.path.span(),
                            "table name already set. static str and function cannot be used both",
                        ));
                    }
                    table_name = Some(TableName::Function(table.value()?.parse()?));
                }
                if table.path.is_ident(KEY_RENAME) {
                    case = table.value()?.parse()?;
                }
                Ok(())
            })?;
        }
    }
    let table_name = table_name.unwrap_or(TableName::StaticStr(LitStr::new(
        &to_snake_case(&id.to_string()),
        id.span(),
    )));
    Ok(TableMeta { table_name, case })
}

fn get_attribute_types_containers<'a>(
    ds: &'a DataStruct,
    to_attribute_ident: &'a TokenStream,
    from_attribute_ident: &'a TokenStream,
    case: Case,
) -> Result<Vec<Container<'a>>> {
    let mut containers = vec![];

    for field in &ds.fields {
        let ident = field
            .ident
            .as_ref()
            .ok_or(Error::new(field.ident.span(), "field ident not found"))?;
        let ty = &field.ty;
        let container = Container::new(ident, ty, to_attribute_ident);
        let (mut container, attribute_value_type) =
            expand_attribute_value(ident, from_attribute_ident, ty, 0, case, container)?;

        parse_from_dynamo_attrs(&field.attrs, field, attribute_value_type, &mut container)?;

        containers.push(container);
    }

    Ok(containers)
}

fn expand_get_table_name_fn(table_name: &TableName) -> TokenStream {
    let table_name_quote = table_name.table_name_quote();
    quote! {
        fn get_table_name() -> &'static ::std::primitive::str {
            #table_name_quote
        }
    }
}

fn expand_prelude_structs(
    vis: &Visibility,
    struct_name: &Ident,
    containers: &[Container],
) -> Vec<TokenStream> {
    let primary_key_fields = containers
        .iter()
        .filter(|c| !c.key_schemas.is_empty())
        .map(|c| {
            let ident = c.field_ident;
            let ty = c.ty;
            quote! { pub #ident: #ty }
        })
        .collect::<Vec<_>>();

    let primary_key_input_struct_name =
        format_ident!("{struct_name}{PRIMARY_KEY_INPUT_STRUCT_POSTFIX}",);
    let primary_key_input_struct = quote! {
        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
        #vis struct #primary_key_input_struct_name {
            #( #primary_key_fields, )*
        }
    };

    vec![primary_key_input_struct]
}

fn expand_create_table_fn(
    containers: &[Container],
    table_name: &TableName,
    case: Case,
    span: Span,
) -> Result<TokenStream> {
    let attribute_definitions_token_stream = containers
        .iter()
        .flat_map(|c| {
            c.attribute_definitions
                .iter()
                .map(|ad| ad.expand_attribute_definition(c.field_ident, case))
        })
        .collect::<Vec<_>>();

    let mut key_schemas = containers
        .iter()
        .flat_map(|c| c.key_schemas.iter().map(|ks| (c.field_ident, ks)))
        .collect::<Vec<_>>();

    validate_and_sort_key_schemas(&mut key_schemas, span)?;

    let key_schema_token_stream = key_schemas
        .into_iter()
        .map(|(ident, ty)| expand_key_schema(ident, *ty, case))
        .collect::<Vec<_>>();

    let table_name_quote = table_name.table_name_quote();

    Ok(quote! {
        fn create_table(
                mut builder: ::aws_sdk_dynamodb::operation::create_table::builders::CreateTableFluentBuilder
            ) -> ::aws_sdk_dynamodb::operation::create_table::builders::CreateTableFluentBuilder {
                builder
                    .table_name(#table_name_quote)
                    #( .attribute_definitions(#attribute_definitions_token_stream) )*
                    #( .key_schema(#key_schema_token_stream) )*
            }
    })
}

fn expand_local_secondary_index_key_schemas_fn(
    attribute_types_containers: &[Container],
    case: Case,
    span: Span,
) -> Result<TokenStream> {
    let mut lsi_key_schema_map = HashMap::<String, Vec<(&Ident, &KeySchemaType)>>::new();
    for container in attribute_types_containers {
        for (index_name, key_schema_types) in &container.local_secondary_index_key_schemas {
            key_schema_types.iter().for_each(|ty| {
                lsi_key_schema_map
                    .entry(index_name.clone())
                    .or_default()
                    .push((container.field_ident, ty));
            });
        }
    }

    lsi_key_schema_map
        .iter_mut()
        .try_for_each(|(_, v)| validate_and_sort_key_schemas(v, span))?;

    let key_schema_token_stream = lsi_key_schema_map
        .into_iter()
        .flat_map(|(index_name, ks)| {
            ks.into_iter().map(move |(ident, key_schema_type)| {
                let lsi_key_schemas_token = expand_key_schema(ident, *key_schema_type, case);
                quote! {
                    lsi_key_schemas.entry(#index_name.to_string()).or_default().push(#lsi_key_schemas_token);
                }
            })
        }).collect::<Vec<_>>();

    Ok(quote! {
        fn get_local_secondary_index_key_schemas()
            -> ::std::collections::HashMap<::std::string::String, Vec<::aws_sdk_dynamodb::types::KeySchemaElement>> {
            let mut lsi_key_schemas: std::collections::HashMap<
                String, Vec<aws_sdk_dynamodb::types::KeySchemaElement>
            > = std::collections::HashMap::new();
            #( #key_schema_token_stream )*;
            lsi_key_schemas
        }
    })
}

fn expand_global_secondary_index_key_schemas_fn(
    attribute_types_containers: &[Container],
    case: Case,
    span: Span,
) -> Result<TokenStream> {
    let mut gsi_key_schema_map = HashMap::<String, Vec<(&Ident, &KeySchemaType)>>::new();
    for container in attribute_types_containers {
        for (index_name, key_schema_types) in &container.global_secondary_index_key_schemas {
            key_schema_types.iter().for_each(|ty| {
                gsi_key_schema_map
                    .entry(index_name.clone())
                    .or_default()
                    .push((container.field_ident, ty));
            });
        }
    }

    gsi_key_schema_map
        .iter_mut()
        .try_for_each(|(_, v)| validate_and_sort_key_schemas(v, span))?;

    let key_schema_token_stream = gsi_key_schema_map
        .into_iter()
        .flat_map(|(index_name, ks)| {
            ks.into_iter().map(move |(ident, key_schema_type)| {
                let gsi_key_schemas_token = expand_key_schema(ident, *key_schema_type, case);
                quote! {
                    gsi_key_schemas.entry(#index_name.to_string()).or_default().push(#gsi_key_schemas_token);
                }
            })
        }).collect::<Vec<_>>();

    Ok(quote! {
        fn get_global_secondary_index_key_schemas()
            -> ::std::collections::HashMap<::std::string::String, Vec<::aws_sdk_dynamodb::types::KeySchemaElement>> {
            let mut gsi_key_schemas: std::collections::HashMap<
                String, Vec<aws_sdk_dynamodb::types::KeySchemaElement>
            > = std::collections::HashMap::new();
            #( #key_schema_token_stream )*;
            gsi_key_schemas
        }
    })
}

fn expand_from_attribute_value_fn(
    attribute_types_containers: &[Container],
    from_attribute_ident: &TokenStream,
) -> TokenStream {
    let fields = attribute_types_containers
        .iter()
        .map(|container| {
            let field_ident = container.field_ident;
            let from_attribute_token_stream = &container.from_attribute_token_stream;
            quote! {
                #field_ident: #from_attribute_token_stream
            }
        })
        .collect::<Vec<_>>();

    quote! {
        fn from_attribute_value(
            #from_attribute_ident: &::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue>
        ) -> Result<Self, ::aws_sdk_dynamodb::types::AttributeValue> {
            Ok(Self { #(# fields ), * })
        }
    }
}

fn expand_put_item_fn(
    attribute_types_containers: &[Container],
    table_name: &TableName,
    case: Case,
) -> TokenStream {
    let to_items = attribute_types_containers
        .iter()
        .map(|container| {
            let ident_lit = Literal::string(&case.apply_str(&container.field_ident.to_string()));
            let item = &container.to_attribute_token_stream;
            quote! { item(#ident_lit.to_string(), #item) }
        })
        .collect::<Vec<_>>();

    let table_name_quote = table_name.table_name_quote();

    quote! {
        fn put_item(
            &self,
            mut builder: ::aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder
        ) -> aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder {
            builder
                .table_name(#table_name_quote)
                #( .#to_items )*
        }
    }
}

fn expand_get_primary_keys_fn(
    struct_name: &Ident,
    case: Case,
    containers: &[Container],
) -> Result<TokenStream> {
    let primary_key_fields = containers
        .iter()
        .filter(|c| !c.key_schemas.is_empty())
        .map(|c| {
            let (_, ty) = expand_attribute_value(
                c.field_ident,
                &c.from_attribute_token_stream,
                c.ty,
                0,
                case,
                c.clone(),
            )?;
            let ident = c.field_ident;
            let ident_to_key = case.apply_str(&ident.to_string());
            Ok(quote! {
                primary_keys.insert(
                    #ident_to_key.to_string(),
                    ::aws_sdk_dynamodb::types::AttributeValue::#ty(input.#ident.to_string())
                );
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let primary_key_input_struct_name =
        format_ident!("{struct_name}{PRIMARY_KEY_INPUT_STRUCT_POSTFIX}");

    Ok(quote! {
        fn get_primary_keys(input: #primary_key_input_struct_name)
        -> ::std::collections::HashMap<
            ::std::string::String, ::aws_sdk_dynamodb::types::AttributeValue>
        {
            let mut primary_keys = ::std::collections::HashMap::new();
            #( #primary_key_fields )*
            primary_keys
        }
    })
}

fn expand_impl_conversions(ident: &Ident, ds: &DataStruct, case: Case) -> Result<Vec<TokenStream>> {
    let to_attribute_ident = quote! { value };
    let from_attribute_ident = quote! { value };

    let containers =
        get_attribute_types_containers(ds, &to_attribute_ident, &from_attribute_ident, case)?;

    container::expand_impl_conversions(ident, &containers, case)
}
