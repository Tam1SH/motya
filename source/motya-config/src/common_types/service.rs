use crate::{
    common_types::{connectors::Connectors, listeners::Listeners, section_parser::SectionParser},
    internal::ProxyConfig,
    kdl::parser::ctx::ParseContext,
};

pub struct ServiceSection<'a, T> {
    listeners: &'a dyn SectionParser<T, Listeners>,
    connectors: &'a dyn SectionParser<T, Connectors>,
    name: &'a str,
}

impl<'a, T> ServiceSection<'a, T> {
    pub fn new(
        listeners: &'a dyn SectionParser<T, Listeners>,
        connectors: &'a dyn SectionParser<T, Connectors>,
        name: &'a str,
    ) -> Self {
        Self {
            listeners,
            connectors,
            name,
        }
    }
}

impl<'a, 'b> SectionParser<ParseContext<'b>, ProxyConfig> for ServiceSection<'a, ParseContext<'b>>
where
    'a: 'b,
{
    fn parse_node(&self, ctx: ParseContext<'b>) -> miette::Result<ProxyConfig> {
        let listeners = self.listeners.parse_node(ctx.clone())?;
        let connectors = self.connectors.parse_node(ctx)?;

        Ok(ProxyConfig {
            name: self.name.to_string(),
            listeners,
            connectors,
        })
    }
}
