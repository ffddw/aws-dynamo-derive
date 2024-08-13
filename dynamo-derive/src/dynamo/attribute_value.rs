use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Error, GenericArgument, PathArguments, Result, Type, TypeArray, TypePath, TypeSlice};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AttributeValueType {
    Blob,
    BlobList,
    Bool,
    List,
    Map,
    Number,
    NumberList,
    Null,
    String,
    StringList,
}

fn get_collection_and_iterator(id: &Ident, depth: usize) -> (TokenStream, Ident) {
    let collection = if depth == 0 {
        quote! { self.#id }
    } else {
        format_ident!("{}private_iterator", "_".repeat(depth)).to_token_stream()
    };
    let iterator = format_ident!("{}private_iterator", "_".repeat(depth + 1));
    (collection, iterator)
}

pub fn expand_attribute_value<'a>(
    to_attribute_id: &'a Ident,
    from_attribute_id: &'a TokenStream,
    ty: &'a Type,
    depth: usize,
    container: AttributeTypesContainer<'a>,
) -> Result<(AttributeTypesContainer<'a>, AttributeValueType)> {
    let (mut container, nested_type) = match ty {
        Type::Path(path) => expand_path(to_attribute_id, from_attribute_id, path, depth, container),
        _ => Err(Error::new(ty.span(), "unsupported type")),
    }?;

    attr_value_types.push(nested_type);

    Ok((token_stream, nested_type))
}

fn expand_path(
    id: &Ident,
    path: &TypePath,
    depth: usize,
    attr_value_types: &mut Vec<AttributeValueType>,
) -> Result<(TokenStream, AttributeValueType)> {
    let (collection, iterator) = get_collection_and_iterator(id, depth);

    let path_segment = path
        .path
        .segments
        .last()
        .ok_or(Error::new(path.span(), "segment not found"))?;

    Ok(match path_segment.ident.to_string().as_str() {
        "Vec" => {
            let abga = match &path_segment.arguments {
                PathArguments::AngleBracketed(abga) => abga,
                _ => {
                    return Err(Error::new(
                        path_segment.arguments.span(),
                        "Vec should be angle bracketed",
                    ))
                }
            };
            match abga
                .args
                .last()
                .ok_or(Error::new(abga.span(), "argument no found"))?
            {
                GenericArgument::Type(ty) => {
                    let (nested_token_stream, nested_type) =
                        expand_attribute_value(id, ty, depth + 1, attr_value_types)?;

                    expand_plural_nested(
                        nested_token_stream,
                        nested_type,
                        collection,
                        iterator,
                        ty.span(),
                    )?
                }
                _ => return Err(Error::new(path_segment.span(), "type expected")),
            }
        }
        "HashMap" => {
            let abga = match &path_segment.arguments {
                PathArguments::AngleBracketed(abga) => abga,
                _ => {
                    return Err(Error::new(
                        path_segment.arguments.span(),
                        "Vec should be angle bracketed",
                    ))
                }
            };

            let key_ty = abga
                .args
                .first()
                .ok_or(Error::new(abga.args.span(), "key type not found"))?;

            let key_type_validation_msg = "key type of HashMap must be String";

            match key_ty {
                GenericArgument::Type(Type::Path(tp)) => {
                    if tp
                        .path
                        .segments
                        .last()
                        .ok_or(Error::new(tp.span(), "argument not found"))?
                        .ident
                        != "String"
                    {
                        return Err(Error::new(
                            tp.path.segments.last().span(),
                            key_type_validation_msg,
                        ));
                    }
                }
                _ => return Err(Error::new(key_ty.span(), key_type_validation_msg)),
            }

            let value_ty = abga
                .args
                .last()
                .ok_or(Error::new(abga.args.span(), "value type not found"))?;

            match value_ty {
                GenericArgument::Type(ty) => {
                    let (nested_token_stream, _) =
                        expand_attribute_value(id, ty, depth + 1, attr_value_types)?;
                    (
                        quote! {
                            {
                                let mut __private_tobe_map = HashMap::new();
                                #collection.iter().for_each(|(__private_key, #iterator)| {
                                    let __nested_value = #nested_token_stream;
                                    __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                                });
                                aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
                            }
                        },
                        AttributeValueType::Map,
                    )
                }
                _ => return Err(Error::new(key_ty.span(), "value type not found")),
            }
        }
        _ => match path_segment.ident.to_string().as_str() {
            "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "i128" | "u128" => (
                quote! {
                    aws_sdk_dynamodb::types::AttributeValue::N(#collection.to_string())
                },
                AttributeValueType::Number,
            ),
            "String" | "str" => (
                quote! {
                    aws_sdk_dynamodb::types::AttributeValue::S(#collection.to_string())
                },
                AttributeValueType::String,
            ),
            "Blob" => (
                quote! {
                    aws_sdk_dynamodb::types::AttributeValue::B(#collection.clone())
                },
                AttributeValueType::Blob,
            ),
            "bool" => (
                quote! {
                    aws_sdk_dynamodb::types::AttributeValue::Bool(#collection)
                },
                AttributeValueType::Bool,
            ),
            "Option" => (
                quote! {
                    aws_sdk_dynamodb::types::AttributeValue::Null( #collection.is_none() )
                },
                AttributeValueType::Null,
            ),
            _ => return Err(Error::new(path_segment.ident.span(), "unsupported type")),
        },
    })
}

fn expand_array(
    id: &Ident,
    array: &TypeArray,
    depth: usize,
    attr_value_types: &mut Vec<AttributeValueType>,
) -> Result<(TokenStream, AttributeValueType)> {
    let (collection, iterator) = get_collection_and_iterator(id, depth);
    let (nested_token_stream, nested_type) =
        expand_attribute_value(id, &array.elem, depth + 1, attr_value_types)?;
    expand_plural_nested(
        nested_token_stream,
        nested_type,
        collection,
        iterator,
        array.elem.span(),
    )
}

fn expand_slice(
    id: &Ident,
    slice: &TypeSlice,
    depth: usize,
    attr_value_types: &mut Vec<AttributeValueType>,
) -> Result<(TokenStream, AttributeValueType)> {
    let (collection, iterator) = get_collection_and_iterator(id, depth);
    let (nested_token_stream, nested_type) =
        expand_attribute_value(id, &slice.elem, depth + 1, attr_value_types)?;
    expand_plural_nested(
        nested_token_stream,
        nested_type,
        collection,
        iterator,
        slice.elem.span(),
    )
}

fn expand_plural_nested(
    nested_token_stream: TokenStream,
    nested_type: AttributeValueType,
    collection: TokenStream,
    iterator: Ident,
    span: Span,
) -> Result<(TokenStream, AttributeValueType)> {
    Ok(match nested_type {
        AttributeValueType::Blob => (
            quote! {
                aws_sdk_dynamodb::types::AttributeValue::Bs(
                    #collection
                        .iter()
                        .map(|#iterator| #iterator.clone())
                        .collect()
                )
            },
            AttributeValueType::BlobList,
        ),

        AttributeValueType::String => (
            quote! {
                aws_sdk_dynamodb::types::AttributeValue::Ss(
                    #collection
                        .iter()
                        .map(|#iterator| #iterator.to_string())
                        .collect()
                )
            },
            AttributeValueType::StringList,
        ),
        AttributeValueType::Number => (
            quote! {
                aws_sdk_dynamodb::types::AttributeValue::Ns(
                    #collection
                        .iter()
                        .map(|#iterator| #iterator.to_string())
                        .collect()
                )
            },
            AttributeValueType::NumberList,
        ),
        AttributeValueType::BlobList
        | AttributeValueType::StringList
        | AttributeValueType::NumberList
        | AttributeValueType::Map
        | AttributeValueType::List => (
            quote! {
                aws_sdk_dynamodb::types::AttributeValue::L(
                    #collection
                        .iter()
                        .map(|#iterator| #nested_token_stream)
                        .collect()
                )
            },
            AttributeValueType::List,
        ),
        _ => return Err(Error::new(span, "unsupported type")),
    })
}

#[cfg(test)]
mod test_attribute_value {
    use crate::dynamo::attribute_value::{expand_attribute_value, AttributeValueType};
    use quote::quote;

    use syn::{parse_quote, Result};

    #[test]
    fn test_simple_types() -> Result<()> {
        let ident = parse_quote! { foo };
        let string_types = [parse_quote! { String }, parse_quote! { &str }];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::S(self.foo.to_string())
        };
        let mut attr_value_types = vec![];
        string_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::String);
            Result::Ok(())
        })?;

        let number_types = [
            parse_quote! { i8 },
            parse_quote! { u8 },
            parse_quote! { i16 },
            parse_quote! { u16 },
            parse_quote! { i32 },
            parse_quote! { i64 },
            parse_quote! { u64 },
            parse_quote! { i128 },
            parse_quote! { u128 },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::N(self.foo.to_string())
        };

        let mut attr_value_types = vec![];
        number_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::Number);
            Result::Ok(())
        })?;

        let mut attr_value_types = vec![];
        let blob_type = parse_quote! { Blob };
        let (ts, root_ty) = expand_attribute_value(&ident, &blob_type, 0, &mut attr_value_types)?;
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::B(self.foo.clone())
        };
        assert_eq!(ts.to_string(), expected.to_string());
        assert_eq!(root_ty, AttributeValueType::Blob);

        let bool_type = parse_quote! { bool };
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::Bool(self.foo)
        };
        let mut attr_value_types = vec![];
        let (ts, root_ty) = expand_attribute_value(&ident, &bool_type, 0, &mut attr_value_types)?;
        assert_eq!(ts.to_string(), expected.to_string());
        assert_eq!(root_ty, AttributeValueType::Bool);

        let null_type = parse_quote! { Option<()> };
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::Null(self.foo.is_none())
        };
        let mut attr_value_types = vec![];
        let (ts, root_ty) = expand_attribute_value(&ident, &null_type, 0, &mut attr_value_types)?;
        assert_eq!(ts.to_string(), expected.to_string());
        assert_eq!(root_ty, AttributeValueType::Null);

        Ok(())
    }

    #[test]
    fn test_list_types() -> Result<()> {
        let ident = parse_quote! { foo };

        let string_list_types = [
            parse_quote! { Vec<String> },
            parse_quote! { Vec<&'a str> },
            parse_quote! { [&'a str; 1] },
            parse_quote! { &'b[str] },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::Ss(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.to_string())
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        string_list_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string(),);
            assert_eq!(root_ty, AttributeValueType::StringList);
            Result::Ok(())
        })?;

        let number_list_types = [
            parse_quote! { Vec<i8> },
            parse_quote! { Vec<&'a u8> },
            parse_quote! { [&'a i16; 1] },
            parse_quote! { &'b[u16] },
            parse_quote! { Vec<i32> },
            parse_quote! { Vec<&'a u32> },
            parse_quote! { [&'a i64; 1] },
            parse_quote! { &'b[u64] },
            parse_quote! { Vec<i128> },
            parse_quote! { Vec<u128> },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::Ns(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.to_string())
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        number_list_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::NumberList);
            Result::Ok(())
        })?;

        let blob_list_types = [parse_quote! { Vec<Blob> }, parse_quote! { &'a [Blob] }];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::Bs(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.clone())
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        blob_list_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::BlobList);
            Result::Ok(())
        })?;

        let nested_number_list_types = [
            parse_quote! { Vec<Vec<u8>> },
            parse_quote! { Vec<[u32; 1]> },
            parse_quote! { Vec<&[u128]> },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::L(
                self.foo
                    .iter()
                    .map(|_private_iterator|
                        aws_sdk_dynamodb::types::AttributeValue::Ns(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator| __private_iterator.to_string())
                                .collect()
                        )
                    )
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        nested_number_list_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::List);
            Result::Ok(())
        })?;

        Ok(())
    }

    #[test]
    fn test_vec_array_slice_combinations() -> Result<()> {
        let ident = parse_quote! { foo };
        let number_combination_types = [
            parse_quote! { &'a [Vec<[i8; 1]>] },
            parse_quote! {&'a [[Vec<u8>; 1]] },
            parse_quote! { Vec<&'a [[i32; 1]]> },
            parse_quote! { Vec<[&'a [u32]; 1]> },
            parse_quote! { [&'a [Vec<i128>]; 1] },
            parse_quote! { [Vec<&'a [u128]>; 1] },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::L(
                self.foo
                    .iter()
                    .map(|_private_iterator|
                        aws_sdk_dynamodb::types::AttributeValue::L(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator|
                                    aws_sdk_dynamodb::types::AttributeValue::Ns(
                                        __private_iterator
                                            .iter()
                                            .map(|___private_iterator| ___private_iterator.to_string())
                                            .collect()
                                    )
                                )
                                .collect()
                        )
                    )
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        number_combination_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::List);
            Result::Ok(())
        })?;

        let string_combination_types = [
            parse_quote! { &'a [Vec<[String; 1]>] },
            parse_quote! {&'a [[Vec<&'a str>; 1]] },
            parse_quote! { Vec<&'a [[String; 1]]> },
            parse_quote! { Vec<[&'a [&'a str]; 1]> },
            parse_quote! { [&'a [Vec<String>]; 1] },
            parse_quote! { [Vec<&'a [&'a str]>; 1] },
        ];
        let expected = quote! {
            aws_sdk_dynamodb::types::AttributeValue::L(
                self.foo
                    .iter()
                    .map(|_private_iterator|
                        aws_sdk_dynamodb::types::AttributeValue::L(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator|
                                    aws_sdk_dynamodb::types::AttributeValue::Ss(
                                        __private_iterator
                                            .iter()
                                            .map(|___private_iterator| ___private_iterator.to_string())
                                            .collect()
                                    )
                                )
                                .collect()
                        )
                    )
                    .collect()
            )
        };
        let mut attr_value_types = vec![];
        string_combination_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::List);
            Result::Ok(())
        })?;

        Ok(())
    }

    #[test]
    fn test_map_not_string_key_fail() {
        let ident = parse_quote! { foo };
        let map = parse_quote! { HashMap<i32, String> };
        let mut attr_value_types = vec![];
        let err = expand_attribute_value(&ident, &map, 0, &mut attr_value_types);
        assert_eq!(
            err.err().unwrap().to_string(),
            "key type of HashMap must be String"
        );
    }

    #[test]
    fn test_map() -> Result<()> {
        let ident = parse_quote! { foo };
        let number_map_types = [
            parse_quote! { HashMap<String, u8> },
            parse_quote! { HashMap<String, u64> },
            parse_quote! { HashMap<String, i128> },
        ];
        let expected = quote! {
            {
                let mut __private_tobe_map = HashMap::new();
                self.foo
                    .iter()
                    .for_each(|(__private_key, _private_iterator)| {
                        let __nested_value =
                            aws_sdk_dynamodb::types::AttributeValue::N(_private_iterator.to_string());
                        __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                    });
                aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
            }
        };

        let mut attr_value_types = vec![];
        number_map_types.iter().try_for_each(|t| {
            let (ts, root_ty) = expand_attribute_value(&ident, t, 0, &mut attr_value_types)?;
            assert_eq!(ts.to_string(), expected.to_string());
            assert_eq!(root_ty, AttributeValueType::Map);
            Result::Ok(())
        })?;

        let nested_map_type = parse_quote! {
            HashMap<String, Vec<HashMap<String, String>>>
        };
        let expected = quote! {
            {
                let mut __private_tobe_map = HashMap::new();
                self.foo
                    .iter()
                    .for_each(|(__private_key, _private_iterator)| {
                        let __nested_value = aws_sdk_dynamodb::types::AttributeValue::L(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator| {
                                    let mut __private_tobe_map = HashMap::new();
                                    __private_iterator.iter().for_each(
                                        |(__private_key, ___private_iterator)| {
                                            let __nested_value = aws_sdk_dynamodb::types::AttributeValue::S(
                                                ___private_iterator.to_string()
                                            );
                                            __private_tobe_map
                                                .insert(__private_key.to_string(), __nested_value);
                                        }
                                    );
                                    aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
                                })
                                .collect()
                        );
                        __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                    });
                aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
            }
        };
        let mut attr_value_types = vec![];
        let (ts, root_ty) =
            expand_attribute_value(&ident, &nested_map_type, 0, &mut attr_value_types)?;
        assert_eq!(ts.to_string(), expected.to_string());
        assert_eq!(root_ty, AttributeValueType::Map);

        Ok(())
    }

    #[test]
    fn test_nested_types() -> Result<()> {
        let ident = parse_quote! { foo };
        let number_map_types = parse_quote! { Vec<HashMap<String, Vec<u32>>> };
        let mut attr_value_types = vec![];
        expand_attribute_value(&ident, &number_map_types, 0, &mut attr_value_types)?;
        assert_eq!(
            attr_value_types,
            vec![
                AttributeValueType::Number,
                AttributeValueType::NumberList,
                AttributeValueType::Map,
                AttributeValueType::List
            ]
        );
        Ok(())
    }
}
