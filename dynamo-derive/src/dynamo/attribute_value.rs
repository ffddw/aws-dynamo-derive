use crate::util::to_pascal_case;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Error, GenericArgument, PathArguments, Result, Type, TypePath};

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

pub struct AttributeTypesContainer<'a> {
    /// field of struct
    pub field_ident: &'a Ident,
    /// type of field
    pub ty: &'a Type,
    /// from Rust type to AttributeValueType
    pub to_attribute_token_stream: TokenStream,
    /// from AttributeValueType to Rust type
    pub from_attribute_token_stream: TokenStream,
}

impl<'a> AttributeTypesContainer<'a> {
    pub fn new(ident: &'a Ident, ty: &'a Type) -> Self {
        Self {
            field_ident: ident,
            ty,
            to_attribute_token_stream: TokenStream::new(),
            from_attribute_token_stream: TokenStream::new(),
        }
    }
}

struct IterVariants {
    to_attribute_collection: TokenStream,
    from_attribute_collection: TokenStream,
    iterator: Ident,
}

fn get_iter_variants(
    field_id: &Ident,
    to_attribute_id: &Ident,
    from_attribute_id: &TokenStream,
    depth: usize,
) -> IterVariants {
    let mut to_attribute_collection =
        format_ident!("{}private_iterator", "_".repeat(depth)).to_token_stream();
    let mut from_attribute_collection = to_attribute_collection.clone();
    let field_id_as_key = Literal::string(&to_pascal_case(&field_id.to_string()));

    if depth == 0 {
        to_attribute_collection = quote! { self.#to_attribute_id };
        from_attribute_collection = quote! {
            #from_attribute_id.get(#field_id_as_key).ok_or(::aws_sdk_dynamodb::types::AttributeValue::Null(true))?
        };
    };

    let iterator = format_ident!("{}private_iterator", "_".repeat(depth + 1));

    IterVariants {
        to_attribute_collection,
        from_attribute_collection,
        iterator,
    }
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

    container.ty = ty;

    Ok((container, nested_type))
}

fn expand_path<'a>(
    to_attribute_id: &'a Ident,
    from_attribute_id: &'a TokenStream,
    path: &'a TypePath,
    depth: usize,
    mut container: AttributeTypesContainer<'a>,
) -> Result<(AttributeTypesContainer<'a>, AttributeValueType)> {
    let iter_variants = get_iter_variants(
        container.field_ident,
        to_attribute_id,
        from_attribute_id,
        depth,
    );
    let IterVariants {
        ref to_attribute_collection,
        ref from_attribute_collection,
        ref iterator,
    } = iter_variants;

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
                    let (container, nested_type) = expand_attribute_value(
                        to_attribute_id,
                        from_attribute_id,
                        ty,
                        depth + 1,
                        container,
                    )?;

                    let (expanded_container, nested_type) =
                        expand_plural_nested(container, nested_type, &iter_variants)?;

                    (expanded_container, nested_type)
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
                    let (mut container, _) = expand_attribute_value(
                        to_attribute_id,
                        from_attribute_id,
                        ty,
                        depth + 1,
                        container,
                    )?;

                    let expanded_to_attribute_token_stream = container.to_attribute_token_stream;
                    container.to_attribute_token_stream = quote! {
                        {
                            let mut __private_tobe_map = HashMap::new();
                            #to_attribute_collection.iter().for_each(|(__private_key, #iterator)| {
                                let __nested_value = #expanded_to_attribute_token_stream;
                                __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                            });
                            ::aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
                        }
                    };

                    let expanded_from_attribute_token_stream =
                        container.from_attribute_token_stream;
                    container.from_attribute_token_stream = quote! {
                        {
                            let mut __private_tobe_map = HashMap::new();
                            #from_attribute_collection.as_m().map_err(|e| e.clone())?
                                .iter()
                                .try_for_each(|(__private_key, #iterator)| {
                                    let __nested_value = #expanded_from_attribute_token_stream;
                                    __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                                    Ok(())
                            })?;
                            Ok(__private_tobe_map)
                        }?
                    };
                    (container, AttributeValueType::Map)
                }
                _ => return Err(Error::new(key_ty.span(), "value type not found")),
            }
        }
        _ => {
            let nested_type = match path_segment.ident.to_string().as_str() {
                "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "i128" | "u128" => {
                    container.to_attribute_token_stream = quote! {
                        ::aws_sdk_dynamodb::types::AttributeValue::N(#to_attribute_collection.to_string())
                    };
                    container.from_attribute_token_stream = quote! {
                        #from_attribute_collection.as_n()
                            .map_err(|e| e.clone())?
                            .parse()
                            .map_err(|_| ::aws_sdk_dynamodb::types::AttributeValue::Null(true))?
                    };
                    AttributeValueType::Number
                }
                "String" => {
                    container.to_attribute_token_stream = quote! {
                        ::aws_sdk_dynamodb::types::AttributeValue::S(#to_attribute_collection.to_string())
                    };
                    container.from_attribute_token_stream = quote! {
                        #from_attribute_collection.as_s().map_err(|e| e.clone())?.to_string()
                    };
                    AttributeValueType::String
                }
                "Blob" => {
                    container.to_attribute_token_stream = quote! {
                        ::aws_sdk_dynamodb::types::AttributeValue::B(#to_attribute_collection.clone())
                    };
                    container.from_attribute_token_stream = quote! {
                        #from_attribute_collection.as_b().map_err(|e| e.clone())?.clone()
                    };
                    AttributeValueType::Blob
                }
                "bool" => {
                    container.to_attribute_token_stream = quote! {
                        ::aws_sdk_dynamodb::types::AttributeValue::Bool(#to_attribute_collection.clone())
                    };
                    container.from_attribute_token_stream = quote! {
                        *#from_attribute_collection.as_bool().map_err(|e| e.clone())?
                    };
                    AttributeValueType::Bool
                }
                "Option" => {
                    container.to_attribute_token_stream = quote! {
                        ::aws_sdk_dynamodb::types::AttributeValue::Null( #to_attribute_collection.is_none() )
                    };
                    container.from_attribute_token_stream = quote! {
                        if *#from_attribute_collection.as_null().map_err(|e| e.clone())? {
                            None
                        } else {
                            Some(())
                        }
                    };
                    AttributeValueType::Null
                }
                _ => return Err(Error::new(path_segment.ident.span(), "unsupported type")),
            };
            (container, nested_type)
        }
    })
}

fn expand_plural_nested<'a>(
    mut container: AttributeTypesContainer<'a>,
    nested_type: AttributeValueType,
    iter_variants: &IterVariants,
) -> Result<(AttributeTypesContainer<'a>, AttributeValueType)> {
    let IterVariants {
        to_attribute_collection,
        from_attribute_collection,
        iterator,
    } = iter_variants;

    let attribute_value_type = match nested_type {
        AttributeValueType::Blob => {
            container.to_attribute_token_stream = quote! {
                ::aws_sdk_dynamodb::types::AttributeValue::Bs(
                    #to_attribute_collection
                        .iter()
                        .map(|#iterator| #iterator.clone())
                        .collect()
                )
            };
            container.from_attribute_token_stream = quote! {
                #from_attribute_collection
                    .as_bs()
                    .map_err(|e| e.clone())?
                    .iter()
                    .map(|#iterator| Ok(#iterator.clone()))
                    .collect::<Result<Vec<_>, _>>()?
            };
            AttributeValueType::BlobList
        }

        AttributeValueType::String => {
            container.to_attribute_token_stream = quote! {
                ::aws_sdk_dynamodb::types::AttributeValue::Ss(
                    #to_attribute_collection
                        .iter()
                        .map(|#iterator| #iterator.to_string())
                        .collect()
                )
            };
            container.from_attribute_token_stream = quote! {
                  #from_attribute_collection
                    .as_ss()
                    .map_err(|e| e.clone())?
                    .iter()
                    .map(|#iterator| Ok(#iterator.to_string()))
                    .collect::<Result<Vec<_>, _>>()?
            };
            AttributeValueType::StringList
        }
        AttributeValueType::Number => {
            container.to_attribute_token_stream = quote! {
                ::aws_sdk_dynamodb::types::AttributeValue::Ns(
                    #to_attribute_collection
                        .iter()
                        .map(|#iterator| #iterator.to_string())
                        .collect()
                )
            };
            container.from_attribute_token_stream = quote! {
                #from_attribute_collection
                    .as_ns()
                    .map_err(|e| e.clone())?
                    .iter()
                    .map(|#iterator| #iterator.parse().map_err(|_| ::aws_sdk_dynamodb::types::AttributeValue::Null(true)))
                    .collect::<Result<Vec<_>, _>>()?
            };
            AttributeValueType::NumberList
        }
        AttributeValueType::BlobList
        | AttributeValueType::List
        | AttributeValueType::Map
        | AttributeValueType::Null
        | AttributeValueType::Bool
        | AttributeValueType::NumberList
        | AttributeValueType::StringList => {
            let nested_to_attribute_token_stream = container.to_attribute_token_stream;
            let nested_from_attribute_token_stream = container.from_attribute_token_stream;
            container.to_attribute_token_stream = quote! {
                ::aws_sdk_dynamodb::types::AttributeValue::L(
                    #to_attribute_collection
                        .iter()
                        .map(|#iterator| #nested_to_attribute_token_stream)
                        .collect()
                )
            };
            container.from_attribute_token_stream = quote! {
                #from_attribute_collection
                    .as_l()
                    .map_err(|e| e.clone())?
                    .iter()
                    .map(|#iterator| Ok(#nested_from_attribute_token_stream))
                    .collect::<Result<Vec<_>, _>>()?
            };
            AttributeValueType::List
        }
    };

    Ok((container, attribute_value_type))
}

#[cfg(test)]
mod test_attribute_value {
    use crate::dynamo::attribute_value::{
        expand_attribute_value, AttributeTypesContainer, AttributeValueType,
    };

    use proc_macro2::{Ident, TokenStream};
    use quote::quote;
    use syn::{parse_quote, Result, Type};
    use test_context::{test_context, TestContext};

    struct AttrValueCtx {
        to_attribute_id: Ident,
        from_attribute_id: TokenStream,
        ty: Type,
    }
    impl TestContext for AttrValueCtx {
        fn setup() -> Self {
            let to_attribute_id = parse_quote! { foo };
            let from_attribute_id = quote! { __private_from_attribute_value };
            let ty = parse_quote! { i32 };

            Self {
                to_attribute_id,
                from_attribute_id,
                ty,
            }
        }
    }

    #[test_context(AttrValueCtx)]
    #[test]
    fn test_simple_types(ctx: &mut AttrValueCtx) -> Result<()> {
        let string_type = parse_quote! { String };
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::S(self.foo.to_string())
        };

        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (container, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &string_type,
            0,
            container,
        )?;
        assert_eq!(
            container.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::String);

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
            ::aws_sdk_dynamodb::types::AttributeValue::N(self.foo.to_string())
        };

        number_types.iter().try_for_each(|t| {
            let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
            let (ts, root_ty) = expand_attribute_value(
                &ctx.to_attribute_id,
                &ctx.from_attribute_id,
                t,
                0,
                container,
            )?;
            assert_eq!(
                ts.to_attribute_token_stream.to_string(),
                expected.to_string()
            );
            assert_eq!(root_ty, AttributeValueType::Number);
            Result::Ok(())
        })?;

        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let blob_type = parse_quote! { Blob };
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &blob_type,
            0,
            container,
        )?;
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::B(self.foo.clone())
        };
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::Blob);

        let bool_type = parse_quote! { bool };
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::Bool(self.foo.clone())
        };
        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &bool_type,
            0,
            container,
        )?;
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::Bool);

        let null_type = parse_quote! { Option<()> };
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::Null(self.foo.is_none())
        };
        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &null_type,
            0,
            container,
        )?;
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::Null);

        Ok(())
    }

    #[test_context(AttrValueCtx)]
    #[test]
    fn test_list_types(ctx: &mut AttrValueCtx) -> Result<()> {
        let string_list_type = parse_quote! { Vec<String> };
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::Ss(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.to_string())
                    .collect()
            )
        };

        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &string_list_type,
            0,
            container,
        )?;
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string(),
        );
        assert_eq!(root_ty, AttributeValueType::StringList);

        let number_list_types = [
            parse_quote! { Vec<i8> },
            parse_quote! { Vec<u8> },
            parse_quote! { Vec<i16> },
            parse_quote! { Vec<i32> },
            parse_quote! { Vec<i128> },
            parse_quote! { Vec<u128> },
        ];
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::Ns(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.to_string())
                    .collect()
            )
        };

        number_list_types.iter().try_for_each(|t| {
            let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
            let (ts, root_ty) = expand_attribute_value(
                &ctx.to_attribute_id,
                &ctx.from_attribute_id,
                t,
                0,
                container,
            )?;
            assert_eq!(
                ts.to_attribute_token_stream.to_string(),
                expected.to_string()
            );
            assert_eq!(root_ty, AttributeValueType::NumberList);
            Result::Ok(())
        })?;

        let blob_list_type = parse_quote! { Vec<Blob> };
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::Bs(
                self.foo
                    .iter()
                    .map(|_private_iterator| _private_iterator.clone())
                    .collect()
            )
        };

        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &blob_list_type,
            0,
            container,
        )?;
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::BlobList);

        let nested_number_list_types = [
            parse_quote! { Vec<Vec<u8>> },
            parse_quote! { Vec<Vec<i32>> },
            parse_quote! { Vec<Vec<u128>> },
        ];
        let expected = quote! {
            ::aws_sdk_dynamodb::types::AttributeValue::L(
                self.foo
                    .iter()
                    .map(|_private_iterator|
                        ::aws_sdk_dynamodb::types::AttributeValue::Ns(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator| __private_iterator.to_string())
                                .collect()
                        )
                    )
                    .collect()
            )
        };

        nested_number_list_types.iter().try_for_each(|t| {
            let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
            let (ts, root_ty) = expand_attribute_value(
                &ctx.to_attribute_id,
                &ctx.from_attribute_id,
                t,
                0,
                container,
            )?;
            assert_eq!(
                ts.to_attribute_token_stream.to_string(),
                expected.to_string()
            );
            assert_eq!(root_ty, AttributeValueType::List);
            Result::Ok(())
        })?;

        Ok(())
    }

    #[test_context(AttrValueCtx)]
    #[test]
    fn test_map_not_string_key_fail(ctx: &mut AttrValueCtx) {
        let map = parse_quote! { HashMap<i32, String> };
        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let err = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &map,
            0,
            container,
        );
        assert_eq!(
            err.err().unwrap().to_string(),
            "key type of HashMap must be String"
        );
    }

    #[test_context(AttrValueCtx)]
    #[test]
    fn test_map(ctx: &mut AttrValueCtx) -> Result<()> {
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
                            ::aws_sdk_dynamodb::types::AttributeValue::N(_private_iterator.to_string());
                        __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                    });
                ::aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
            }
        };

        number_map_types.iter().try_for_each(|t| {
            let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
            let (ts, root_ty) = expand_attribute_value(
                &ctx.to_attribute_id,
                &ctx.from_attribute_id,
                t,
                0,
                container,
            )?;
            assert_eq!(
                ts.to_attribute_token_stream.to_string(),
                expected.to_string()
            );
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
                        let __nested_value = ::aws_sdk_dynamodb::types::AttributeValue::L(
                            _private_iterator
                                .iter()
                                .map(|__private_iterator| {
                                    let mut __private_tobe_map = HashMap::new();
                                    __private_iterator.iter().for_each(
                                        |(__private_key, ___private_iterator)| {
                                            let __nested_value = ::aws_sdk_dynamodb::types::AttributeValue::S(
                                                ___private_iterator.to_string()
                                            );
                                            __private_tobe_map
                                                .insert(__private_key.to_string(), __nested_value);
                                        }
                                    );
                                    ::aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
                                })
                                .collect()
                        );
                        __private_tobe_map.insert(__private_key.to_string(), __nested_value);
                    });
                ::aws_sdk_dynamodb::types::AttributeValue::M(__private_tobe_map)
            }
        };
        let container = AttributeTypesContainer::new(&ctx.to_attribute_id, &ctx.ty);
        let (ts, root_ty) = expand_attribute_value(
            &ctx.to_attribute_id,
            &ctx.from_attribute_id,
            &nested_map_type,
            0,
            container,
        )?;
        assert_eq!(
            ts.to_attribute_token_stream.to_string(),
            expected.to_string()
        );
        assert_eq!(root_ty, AttributeValueType::Map);

        Ok(())
    }
}
