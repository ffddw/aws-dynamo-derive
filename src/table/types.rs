use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, LitStr};

pub enum TableName {
    StaticStr(LitStr),
    Function(Expr),
}

impl TableName {
    pub fn table_name_quote(&self) -> TokenStream {
        match self {
            Self::StaticStr(tn) => quote! { #tn },
            Self::Function(tn_name_fn) => quote! { (#tn_name_fn)() },
        }
    }
}
