use fqdn::FQDN;
use kdl::{KdlDocument, KdlEntry, KdlNode};
use miette::{Result, SourceSpan};
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Range, RangeFrom, RangeFull, RangeTo},
    str::FromStr,
    vec::IntoIter,
};

use crate::{common_types::bad::Bad, kdl::parser::typed_value::TypedValue};

#[derive(Debug, Clone)]
pub struct ParseContext<'a> {
    pub doc: &'a KdlDocument,
    pub source_name: &'a str,
    pub current: Current<'a>,
}

#[derive(Debug, Clone)]
pub enum Current<'a> {
    Document(&'a KdlDocument),
    Node(&'a KdlNode, &'a [KdlEntry]),
}

impl<'a> ParseContext<'a> {
    /// Creates a new parsing context from a document and a specific location (node or root).
    pub fn new(doc: &'a KdlDocument, current: Current<'a>, source_name: &'a str) -> Self {
        Self {
            doc,
            source_name,
            current,
        }
    }

    /// Creates a new context for the child block's content.
    /// Returns an error if the block does not exist.
    pub fn enter_block(&self) -> Result<ParseContext<'a>> {
        match &self.current {
            Current::Node(node, _) => {
                let children = node.children().ok_or_else(|| {
                    self.error("Expected a children block { ... }, but none found")
                })?;

                Ok(ParseContext::new(
                    self.doc,
                    Current::Document(children),
                    self.source_name,
                ))
            }
            Current::Document(_) => {
                Err(self.error("Cannot enter block: current context is already a document root"))
            }
        }
    }

    /// Creates a new context focused on a specific child node.
    pub fn for_node(&self, node: &'a KdlNode, args: &'a [KdlEntry]) -> Self {
        Self {
            current: Current::Node(node, args),
            ..self.clone()
        }
    }

    pub fn error_with_span(&self, msg: impl Into<String>, span: SourceSpan) -> miette::Error {
        Bad::docspan(msg.into(), self.doc, &span, self.source_name).into()
    }

    /// Generates a styled error message pointing to the current span in the source.
    pub fn error(&self, msg: impl Into<String>) -> miette::Error {
        Bad::docspan(msg.into(), self.doc, &self.current_span(), self.source_name).into()
    }

    /// Returns the source span of the current element (Node or Document).
    pub fn current_span(&self) -> SourceSpan {
        match &self.current {
            Current::Document(doc) => doc.span(),
            Current::Node(node, _) => node.span(),
        }
    }

    /// Returns the name of the current node (e.g., "server" in `server "localhost"`).
    /// Returns an error if the context is the Document root.
    pub fn name(&self) -> Result<&str> {
        match &self.current {
            Current::Document(_) => Err(self.error("Expected node, but current is a document")),
            Current::Node(node, _) => Ok(node.name().value()),
        }
    }

    pub fn nodes_iter<'b>(&self) -> Result<IntoIter<ParseContext<'_>>>
    where
        'a: 'b,
    {
        Ok(self.nodes()?.into_iter())
    }
    /// Iterates over child nodes, returning a new `ParseContext` for each child.
    pub fn nodes<'b>(&self) -> Result<Vec<ParseContext<'b>>>
    where
        'a: 'b,
    {
        let doc = match self.current {
            Current::Document(d) => d,
            Current::Node(n, _) => n
                .children()
                .ok_or_else(|| self.error("Expected children block"))?,
        };

        let nodes = doc
            .nodes()
            .iter()
            .map(|node| (node, node.name().value(), node.entries()));

        Ok(nodes
            .map(|(node, _name, args)| ParseContext {
                current: Current::Node(node, args),
                ..self.clone()
            })
            .collect())
    }

    /// Asserts that the current node has a specific name.
    pub fn expect_name(&self, expected: &str) -> Result<()> {
        match &self.current {
            Current::Document(_) => Err(self.error(format!(
                "Expected node '{expected}', but current is a document"
            ))),
            Current::Node(node, _) => {
                if node.name().value() == expected {
                    Ok(())
                } else {
                    Err(self.error(format!(
                        "Expected '{expected}', found '{}'",
                        node.name().value()
                    )))
                }
            }
        }
    }

    /// Returns the raw slice of arguments/entries for the current node.
    pub fn args(&self) -> Result<&[KdlEntry]> {
        match &self.current {
            Current::Document(_) => Err(self.error("Expected node, but current is a document")),
            Current::Node(_, args) => Ok(args),
        }
    }

    /// Extracts named arguments into a map and enforces a whitelist of allowed keys.
    pub fn args_map_with_only_keys<R>(
        &self,
        range: R,
        allowed: &[&str],
    ) -> Result<HashMap<&str, &str>>
    where
        R: SliceRange<[KdlEntry]>,
    {
        self.args_map(range)?.ensure_only_keys(
            allowed,
            self.doc,
            &self.current_span(),
            self.source_name,
        )
    }

    /// Extracts named arguments (key="value") into a HashMap within a specific range.
    pub fn args_map<R>(&self, range: R) -> Result<HashMap<&str, &str>>
    where
        R: SliceRange<[KdlEntry]>,
    {
        let args = self.args()?;
        let sliced = range
            .slice(args)
            .ok_or_else(|| self.error("Range out of bounds"))?;

        Ok(sliced
            .iter()
            .filter_map(|arg| {
                let name = arg.name()?.value();
                let value = arg.value().as_string()?;
                Some((name, value))
            })
            .collect())
    }

    /// Retrieves a required named property as a String.
    pub fn string_arg(&self, name: &str) -> Result<String> {
        let entry = self
            .opt_prop(name)?
            .ok_or_else(|| self.error(format!("Missing required argument: '{name}'")))?;

        Ok(entry.as_str()?.to_string())
    }

    /// Retrieves a required named property and parses it as an FQDN.
    pub fn parse_fqdn_arg(&self, name: &str) -> Result<FQDN> {
        let str = self.string_arg(name)?;
        FQDN::from_str(&str).map_err(|err| self.error(format!("Invalid FQDN '{str}': {err}")))
    }

    /// Checks if the current node has an attached children block (e.g., `{ ... }`).
    pub fn has_children_block(&self) -> Result<bool> {
        match &self.current {
            Current::Node(n, _) => Ok(n.children().is_some()),
            Current::Document(_) => Err(self.error("Expected node, but current is a document")),
        }
    }

    /// Retrieves child nodes but returns an error if the block is empty.
    pub fn req_nodes(&self) -> Result<Vec<ParseContext<'_>>> {
        let nodes = self.nodes()?;

        if nodes.is_empty() {
            return Err(self.error(format!(
                "Block '{name}' cannot be empty",
                name = self.name()?
            )));
        }

        Ok(nodes)
    }

    pub fn props<'b, const N: usize>(
        &'a self,
        keys: [&str; N],
    ) -> Result<[Option<TypedValue<'b>>; N]>
    where
        'a: 'b,
    {
        let mut result = [None; N];

        for (i, key) in keys.iter().enumerate() {
            result[i] = self.opt_prop(key)?;
        }

        Ok(result)
    }
}
pub trait SliceRange<T: ?Sized> {
    fn slice<'a>(&self, slice: &'a T) -> Option<&'a T>;
}

impl<T> SliceRange<[T]> for Range<usize> {
    fn slice<'a>(&self, slice: &'a [T]) -> Option<&'a [T]> {
        slice.get(self.start..self.end)
    }
}

impl<T> SliceRange<[T]> for RangeFrom<usize> {
    fn slice<'a>(&self, slice: &'a [T]) -> Option<&'a [T]> {
        slice.get(self.start..)
    }
}

impl<T> SliceRange<[T]> for RangeTo<usize> {
    fn slice<'a>(&self, slice: &'a [T]) -> Option<&'a [T]> {
        slice.get(..self.end)
    }
}

impl<T> SliceRange<[T]> for RangeFull {
    fn slice<'a>(&self, slice: &'a [T]) -> Option<&'a [T]> {
        Some(slice)
    }
}

pub trait HashMapValidationExt {
    fn ensure_only_keys(
        self,
        allowed: &[&str],
        doc: &KdlDocument,
        span: &SourceSpan,
        source_name: &str,
    ) -> miette::Result<Self>
    where
        Self: Sized;
}

impl<V> HashMapValidationExt for HashMap<&str, V> {
    fn ensure_only_keys(
        self,
        allowed: &[&str],
        doc: &KdlDocument,
        span: &SourceSpan,
        source_name: &str,
    ) -> miette::Result<Self> {
        if let Some(bad_key) = self.keys().find(|k| !allowed.contains(k)) {
            return Err(Bad::docspan(
                format!(
                    "Unknown configuration key: '{bad_key}'. Allowed keys are: {:?}",
                    allowed
                ),
                doc,
                span,
                source_name,
            )
            .into());
        }

        Ok(self)
    }
}
