use crate::container::Container;
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::util::to_pascal_case;

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Result};

pub fn expand_item(input: &mut DeriveInput) -> Result<TokenStream> {
    let DeriveInput { ident, data, .. } = input;

    let ds = match &data {
        Data::Struct(ds) => ds,
        _ => return Err(Error::new(input.span(), "only struct type available")),
    };

    let to_attribute_ident = quote! { value };
    let from_attribute_ident = quote! { value };
    let containers =
        get_attribute_types_containers(ds, &to_attribute_ident, &from_attribute_ident)?;
    let impl_conversions = expand_impl_conversions(ident, &containers);

    Ok(quote! {
        #impl_conversions
    })
}

fn get_attribute_types_containers<'a>(
    ds: &'a DataStruct,
    to_attribute_ident: &'a TokenStream,
    from_attribute_ident: &'a TokenStream,
) -> Result<Vec<Container<'a>>> {
    let mut containers = vec![];

    for field in &ds.fields {
        let ty = &field.ty;
        let ident = field
            .ident
            .as_ref()
            .ok_or(Error::new(field.ident.span(), "field ident not found"))?;

        let container = Container::new(ident, ty, to_attribute_ident);
        let (container, _) = expand_attribute_value(ident, from_attribute_ident, ty, 0, container)?;
        containers.push(container);
    }

    Ok(containers)
}

fn expand_impl_conversions(ident: &Ident, containers: &[Container]) -> TokenStream {
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

    quote! {
        impl From<&#ident> for ::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue> {
            fn from(value: &#ident) -> Self {
                let mut map = ::std::collections::HashMap::new();
                #( #map_inserts )*
                map
            }
        }

        impl TryFrom<&::std::collections::HashMap<
            ::std::string::String,
            ::aws_sdk_dynamodb::types::AttributeValue,
        >> for #ident {
            type Error = ::aws_sdk_dynamodb::types::AttributeValue;
            fn try_from(value: &::std::collections::HashMap<
                ::std::string::String,
                ::aws_sdk_dynamodb::types::AttributeValue>
            ) -> Result<Self, Self::Error> {
                Ok(Self { #(# from_attr_fields ), * })
            }
        }
    }
}
