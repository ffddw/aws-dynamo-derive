use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote;
use std::fmt::{Display, Formatter};
use syn::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum KeySchemaType {
    HashKey,
    RangeKey,
}

impl Display for KeySchemaType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::HashKey => "hash_key",
            Self::RangeKey => "range_key",
        };
        write!(f, "{val}")
    }
}

pub fn expand_key_schema(id: &Ident, key_type: KeySchemaType) -> TokenStream {
    let ident = Literal::string(&to_pascal_case(&id.to_string()));

    let key_type = match key_type {
        KeySchemaType::HashKey => quote! { ::aws_sdk_dynamodb::types::KeyType::Hash },
        KeySchemaType::RangeKey => quote! { ::aws_sdk_dynamodb::types::KeyType::Range },
    };

    quote! {
        aws_sdk_dynamodb::types::KeySchemaElement::builder()
            .attribute_name(#ident.to_string())
            .key_type(#key_type)
            .build()
            .unwrap()
    }
}

pub fn validate_and_sort_key_schemas(
    key_schemas: &mut [(&Ident, &KeySchemaType)],
    span: Span,
) -> syn::Result<()> {
    match key_schemas
        .iter_mut()
        .filter(|(_, ks)| ks.eq(&&KeySchemaType::HashKey))
        .count()
    {
        0 => Err(Error::new(span, "HashKey not found")),
        2.. => Err(Error::new(span, "only one HashKey is allowed")),
        1 => Ok(()),
    }?;

    if key_schemas
        .iter()
        .filter(|(_, ks)| ks.eq(&&KeySchemaType::RangeKey))
        .count()
        > 1
    {
        return Err(Error::new(span, "at most one RangeKey is allowed"));
    };

    key_schemas.sort_by_key(|(_, k)| *k);

    Ok(())
}

#[cfg(test)]
mod test_key_schema {
    use crate::dynamo::key_schema::KeySchemaType;

    #[test]
    fn sort_key_schema() {
        let mut key_schemas = vec![
            KeySchemaType::HashKey,
            KeySchemaType::RangeKey,
            KeySchemaType::HashKey,
            KeySchemaType::RangeKey,
        ];

        key_schemas.sort();
        assert_eq!(
            key_schemas,
            vec![
                KeySchemaType::HashKey,
                KeySchemaType::HashKey,
                KeySchemaType::RangeKey,
                KeySchemaType::RangeKey,
            ]
        );
    }
}
