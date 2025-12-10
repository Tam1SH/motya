use std::{fmt::Display, str::FromStr};

use kdl::{KdlEntry, KdlValue};
use miette::Result;

use crate::kdl::parser::{ctx::ParseContext, utils::get_simple_type_name};

#[derive(Clone, Copy)]
pub struct TypedValue<'a> {
    ctx: &'a ParseContext<'a>,
    entry: &'a KdlEntry,
}

impl<'a> TypedValue<'a> {
    pub fn new(ctx: &'a ParseContext<'a>, entry: &'a KdlEntry) -> Self {
        Self { ctx, entry }
    }

    pub fn as_str(self) -> Result<String> {
        self.entry
            .value()
            .as_string()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                self.ctx.error_with_span(
                    format!("Expected a string value, found {:?}", self.entry.value()),
                    self.entry.span(),
                )
            })
    }

    pub fn as_usize(self) -> Result<usize> {
        self.entry
            .value()
            .as_integer()
            .and_then(|i| usize::try_from(i).ok())
            .ok_or_else(|| {
                self.ctx.error_with_span(
                    format!(
                        "Expected a positive integer, found {:?}",
                        self.entry.value()
                    ),
                    self.entry.span(),
                )
            })
    }

    pub fn as_bool(self) -> Result<bool> {
        self.entry.value().as_bool().ok_or_else(|| {
            self.ctx.error_with_span(
                format!("Expected a boolean, found {:?}", self.entry.value()),
                self.entry.span(),
            )
        })
    }

    pub fn parse_as<T>(self) -> Result<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        let raw_str = self.as_string_lossy()?;
        T::from_str(&raw_str).map_err(|e| {
            let type_name = get_simple_type_name::<T>();
            self.ctx.error_with_span(
                format!("Invalid {type_name} '{raw_str}'. Reason: {e}"),
                self.entry.span(),
            )
        })
    }

    pub fn as_string_lossy(self) -> Result<String> {
        match self.entry.value() {
            KdlValue::String(s) => Ok(s.clone()),
            KdlValue::Integer(i) => Ok(i.to_string()),
            KdlValue::Float(f) => Ok(f.to_string()),
            KdlValue::Bool(b) => Ok(b.to_string()),

            KdlValue::Null => Err(self.ctx.error_with_span(
                "Cannot parse 'null' as a string or number",
                self.entry.span(),
            )),
        }
    }
}

impl<'a> ParseContext<'a> {
    pub fn first<'b>(&'a self) -> Result<TypedValue<'b>>
    where
        'a: 'b,
    {
        let entry = self
            .args()?
            .first()
            .ok_or_else(|| self.error("Missing required first argument"))?;

        Ok(TypedValue::new(self, entry))
    }

    pub fn arg<'b>(&'a self, index: usize) -> Result<TypedValue<'b>>
    where
        'a: 'b,
    {
        let entry = self
            .args()?
            .iter()
            .filter(|e| e.name().is_none())
            .nth(index)
            .ok_or_else(|| {
                self.error(format!(
                    "Missing required argument at position {}",
                    index + 1
                ))
            })?;

        Ok(TypedValue::new(self, entry))
    }

    pub fn prop<'b>(&'a self, key: &str) -> Result<TypedValue<'b>>
    where
        'a: 'b,
    {
        let entry = self
            .args()?
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some(key))
            .ok_or_else(|| self.error(format!("Missing required property '{}'", key)))?;

        Ok(TypedValue::new(self, entry))
    }

    pub fn opt_prop<'b>(&'a self, key: &str) -> Result<Option<TypedValue<'b>>>
    where
        'a: 'b,
    {
        let entry = self
            .args()?
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some(key));

        Ok(entry.map(|e| TypedValue::new(self, e)))
    }
}
