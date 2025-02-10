use crate::util::to_pascal_case;

use proc_macro2::Span;
use std::str::FromStr;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Result};

#[derive(Default, Copy, Clone)]
pub enum Case {
    #[default]
    SnakeCase,
    PascalCase,
}

impl Case {
    pub fn apply_str(&self, s: &str) -> String {
        match self {
            Self::SnakeCase => s.to_string(),
            Self::PascalCase => to_pascal_case(s),
        }
    }
}

impl FromStr for Case {
    type Err = syn::Error;
    fn from_str(s: &str) -> Result<Self> {
        let case = match s {
            "snake_case" => Self::SnakeCase,
            "PascalCase" => Self::PascalCase,
            _ => {
                return Err(Self::Err::new(
                    Span::call_site(),
                    format!("invalid case: {s}"),
                ))
            }
        };

        Ok(case)
    }
}

impl Parse for Case {
    fn parse(input: ParseStream) -> Result<Self> {
        let i: LitStr = input.parse()?;
        i.value().parse()
    }
}
