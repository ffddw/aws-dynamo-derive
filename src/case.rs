use crate::util::{strip_raw_r, to_pascal_case, to_snake_case};

use proc_macro2::Span;
use std::str::FromStr;
use syn::parse::{Parse, ParseStream};
use syn::{Error, LitStr, Result};

#[derive(Default, Copy, Clone)]
pub enum Case {
    #[default]
    SnakeCase,
    PascalCase,
}

impl Case {
    pub fn apply_str(&self, s: &str) -> String {
        let s = strip_raw_r(s);
        match self {
            Self::SnakeCase => to_snake_case(s),
            Self::PascalCase => to_pascal_case(s),
        }
    }
}

impl FromStr for Case {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let case = match s {
            "snake_case" => s.parse()?,
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
