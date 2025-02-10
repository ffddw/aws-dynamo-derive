use crate::case::Case;
use crate::container::{expand_impl_conversions, Container};
use crate::dynamo::attribute_value::expand_attribute_value;
use crate::tags::{AWS_DYNAMO_ATTR_META_ENTRY, KEY_RENAME};

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Data, DataStruct, DeriveInput, Error, Result};

struct ItemMeta {
    case: Case,
}

pub fn expand_item(input: &mut DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        ident, data, attrs, ..
    } = input;

    let ds = match &data {
        Data::Struct(ds) => ds,
        _ => return Err(Error::new(input.span(), "only struct type available")),
    };

    let ItemMeta { case } = get_item_meta(attrs)?;

    let to_attribute_ident = quote! { value };
    let from_attribute_ident = quote! { value };
    let containers =
        get_attribute_types_containers(ds, &to_attribute_ident, &from_attribute_ident, case)?;
    let impl_conversions = expand_impl_conversions(ident, &containers, case)?;

    Ok(quote! {

        #(
            #[allow(clippy::needless_question_mark)]
            #impl_conversions
        )*
    })
}

fn get_item_meta(attrs: &[Attribute]) -> Result<ItemMeta> {
    let mut case = Case::default();
    for attr in attrs {
        if attr.path().is_ident(AWS_DYNAMO_ATTR_META_ENTRY) {
            attr.parse_nested_meta(|table| {
                if table.path.is_ident(KEY_RENAME) {
                    case = table.value()?.parse()?;
                }
                Ok(())
            })?;
        }
    }
    Ok(ItemMeta { case })
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
        let (container, _) =
            expand_attribute_value(ident, from_attribute_ident, ty, 0, case, container)?;
        containers.push(container);
    }

    Ok(containers)
}
