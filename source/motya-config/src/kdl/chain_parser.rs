use crate::{
    common_types::definitions::{ConfiguredFilter, FilterChain},
    kdl::parser::{block::BlockParser, ctx::ParseContext, ensures::Rule},
};
use std::collections::HashMap;

pub struct ChainParser;

impl ChainParser {
    pub fn parse(&self, ctx: ParseContext<'_>) -> miette::Result<FilterChain> {
        let mut block = BlockParser::new(ctx)?;
        let filters = block.repeated("filter", |filter_ctx| {
            filter_ctx.validate(&[Rule::NoChildren, Rule::NoPositionalArgs])?;

            let name = filter_ctx.prop("name")?.parse_as::<fqdn::FQDN>()?;

            let all_args = filter_ctx.args_map(1..)?;

            let args = all_args
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>();

            Ok(ConfiguredFilter { name, args })
        })?;

        block.exhaust()?;

        Ok(FilterChain { filters })
    }
}

#[cfg(test)]
mod tests {
    use crate::kdl::parser::ctx::Current;

    use super::*;
    use kdl::KdlDocument;

    #[test]
    fn test_chain_parser_success_happy_path() {
        let kdl_input = r#"
            filter name="com.example.auth"
            filter name="com.example.logger" level="debug" format="json"
        "#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let chain = ChainParser.parse(ctx).expect("Should parse valid chain");

        assert_eq!(chain.filters.len(), 2);

        let f1 = &chain.filters[0];
        assert_eq!(f1.name.to_string(), "com.example.auth");
        assert!(f1.args.is_empty());

        let f2 = &chain.filters[1];
        assert_eq!(f2.name.to_string(), "com.example.logger");
        assert_eq!(f2.args.get("level").unwrap(), "debug");
        assert_eq!(f2.args.get("format").unwrap(), "json");
    }

    #[test]
    fn test_chain_parser_empty_block() {
        let kdl_input = "";
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let chain = ChainParser.parse(ctx).expect("Should parse valid chain");
        assert!(chain.filters.is_empty());
    }

    #[test]
    fn test_chain_parser_invalid_directive_name() {
        let kdl_input = r#"
            filter name="good.filter"
            not-filter name="bad.one"
        "#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let result = ChainParser.parse(ctx);
        let msg_err = result.unwrap_err().help().unwrap().to_string();

        crate::assert_err_contains!(msg_err, "Unknown directive: 'not-filter'");
    }

    #[test]
    fn test_chain_parser_missing_name_argument() {
        let kdl_input = r#"
            filter arg="value"
        "#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let result = ChainParser.parse(ctx);
        let msg_err = result.unwrap_err().help().unwrap().to_string();
        crate::assert_err_contains!(msg_err, "Missing required property 'name'");
    }

    #[test]
    fn test_chain_parser_invalid_fqdn() {
        let kdl_input = r#"
            filter name="invalid name with spaces"
        "#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let result = ChainParser.parse(ctx);
        let msg_err = result.unwrap_err().help().unwrap().to_string();

        crate::assert_err_contains!(
            msg_err,
            "Invalid FQDN 'invalid name with spaces'. Reason: invalid char found in FQDN"
        );
    }
}
