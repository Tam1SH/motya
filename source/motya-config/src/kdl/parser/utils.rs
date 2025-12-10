use std::{any::type_name, fmt::Display, str::FromStr};

use kdl::KdlValue;
use miette::Result;

use crate::kdl::parser::typed_value::TypedValue;

#[allow(clippy::wrong_self_convention)]
pub trait OptionTypedValueExt {
    fn as_str(self) -> Result<Option<String>>;
    fn as_bool(self) -> Result<Option<bool>>;
    fn as_usize(self) -> Result<Option<usize>>;
    fn parse_as<T>(self) -> Result<Option<T>>
    where
        T: FromStr,
        T::Err: Display;
}

impl<'a> OptionTypedValueExt for Option<TypedValue<'a>> {
    fn as_str(self) -> Result<Option<String>> {
        match self {
            Some(v) => Ok(Some(v.as_str()?)),
            None => Ok(None),
        }
    }

    fn as_bool(self) -> Result<Option<bool>> {
        match self {
            Some(v) => Ok(Some(v.as_bool()?)),
            None => Ok(None),
        }
    }

    fn as_usize(self) -> Result<Option<usize>> {
        match self {
            Some(v) => Ok(Some(v.as_usize()?)),
            None => Ok(None),
        }
    }
    fn parse_as<T>(self) -> Result<Option<T>>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self {
            Some(v) => Ok(Some(v.parse_as::<T>()?)),
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    String,
    Integer,
    Float,
    Bool,
    Null,
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::String => write!(f, "String"),
            PrimitiveType::Integer => write!(f, "Integer"),
            PrimitiveType::Float => write!(f, "Float"),
            PrimitiveType::Bool => write!(f, "Boolean"),
            PrimitiveType::Null => write!(f, "Null"),
        }
    }
}

pub fn get_simple_type_name<T>() -> &'static str {
    type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("UnknownType")
}

pub fn get_kdl_type_name(val: &KdlValue) -> &'static str {
    match val {
        KdlValue::String(_) => "String",
        KdlValue::Integer(_) => "Integer",
        KdlValue::Float(_) => "Float",
        KdlValue::Bool(_) => "Boolean",
        KdlValue::Null => "Null",
    }
}
