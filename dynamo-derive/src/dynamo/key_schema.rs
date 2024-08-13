use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum KeySchemaType {
    HashKey,
    RangeKey,
}

impl PartialOrd for KeySchemaType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KeySchemaType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (KeySchemaType::HashKey, KeySchemaType::HashKey) => std::cmp::Ordering::Equal,
            (KeySchemaType::RangeKey, KeySchemaType::RangeKey) => std::cmp::Ordering::Equal,
            (KeySchemaType::HashKey, KeySchemaType::RangeKey) => std::cmp::Ordering::Less,
            (KeySchemaType::RangeKey, KeySchemaType::HashKey) => std::cmp::Ordering::Greater,
        }
    }
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
