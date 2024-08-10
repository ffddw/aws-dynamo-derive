use proc_macro2::{Ident, TokenStream};
use syn::{Fields, Meta, Type};

/// ```rust,ignore
///  #[table(hash_key("S"))]
///  pub name: String,
/// ```
///
/// ty: String, ident: name, ext: "S"
#[derive(Debug)]
pub struct ExtField<'a> {
    pub _ty: &'a Type,
    pub ident: &'a Option<Ident>,
    pub ext: TokenStream,
}

impl<'a> ExtField<'a> {
    pub fn get_paren_kv(fields: &'a Fields, key: &str) -> Vec<Self> {
        let mut res = vec![];

        for field in fields {
            let attr_idents = field
                .attrs
                .iter()
                .filter_map(|attr| {
                    let meta = attr.parse_args::<Meta>().ok()?;
                    if let Meta::List(ml) = &meta {
                        Some((ml.tokens.clone(), meta))
                    } else {
                        None
                    }
                })
                .flat_map(|(ext, m)| {
                    m.path()
                        .segments
                        .iter()
                        .map(|ps| (ps.ident.to_string(), ext.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            attr_idents
                .into_iter()
                .find(|(id, _)| key.eq(id))
                .into_iter()
                .for_each(|(_, ext)| {
                    res.push(Self {
                        _ty: &field.ty,
                        ident: &field.ident,
                        ext,
                    })
                });
        }
        res
    }
}
