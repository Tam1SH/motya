use crate::{
    common_types::definitions::{HashAlgorithm, KeyTemplateConfig, Transform},
    kdl::parser::{block::BlockParser, ctx::ParseContext},
};

pub struct KeyProfileParser;

impl KeyProfileParser {
    pub fn parse(&self, ctx: ParseContext<'_>) -> miette::Result<KeyTemplateConfig> {
        let mut block = BlockParser::new(ctx)?;

        let (source, fallback) = block.required("key", |ctx| {
            let source = ctx.first()?.as_str()?;

            let fallback = ctx
                .args_map_with_only_keys(1.., &["fallback"])?
                .get("fallback")
                .map(|s| s.to_string());

            Ok((source, fallback))
        })?;

        let algorithm = block
            .optional("algorithm", |c| {
                let opts = c.args_map_with_only_keys(.., &["name", "seed"])?;

                Ok(HashAlgorithm {
                    name: opts.get("name").unwrap_or(&"xxhash64").to_string(),
                    seed: opts.get("seed").map(|s| s.to_string()),
                })
            })?
            .unwrap_or_else(|| HashAlgorithm {
                name: "xxhash64".to_string(),
                seed: None,
            });

        let transforms = block
            .optional("transforms-order", |c| {
                let mut steps = Vec::new();
                for step_ctx in c.nodes()? {
                    let name = step_ctx.name().unwrap_or("").to_string();
                    let params = step_ctx
                        .args_map(..)?
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect();

                    steps.push(Transform { name, params });
                }
                Ok(steps)
            })?
            .unwrap_or_default();

        block.exhaust()?;

        Ok(KeyTemplateConfig {
            source,
            fallback,
            algorithm,
            transforms,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::kdl::parser::ctx::Current;

    use super::*;
    use kdl::KdlDocument;

    #[test]
    fn test_parse_key_profile() {
        let kdl_input = r#"
            key "${cookie_session}" fallback="${client_ip}:${user_agent}"
            algorithm name="xxhash32" seed="idk"
            transforms-order {
                remove-query-params
                lowercase
                truncate length="256"
            }
        "#;

        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let template = KeyProfileParser.parse(ctx).expect("Should parse");

        assert_eq!(template.source, "${cookie_session}");
        assert_eq!(
            template.fallback.as_deref(),
            Some("${client_ip}:${user_agent}")
        );
        assert_eq!(template.algorithm.name, "xxhash32");
        assert_eq!(template.algorithm.seed.as_deref(), Some("idk"));

        assert_eq!(template.transforms.len(), 3);
        assert_eq!(template.transforms[0].name, "remove-query-params");
        assert_eq!(template.transforms[1].name, "lowercase");
        assert_eq!(template.transforms[2].name, "truncate");
        assert_eq!(
            template.transforms[2].params.get("length"),
            Some(&"256".to_string())
        );
    }

    #[test]
    fn test_parse_minimal_profile() {
        let kdl_input = r#"key "${uri_path}""#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let template = KeyProfileParser.parse(ctx).unwrap();

        assert_eq!(template.source, "${uri_path}");
        assert!(template.fallback.is_none());
        assert_eq!(template.algorithm.name, "xxhash64");
        assert!(template.algorithm.seed.is_none());
        assert!(template.transforms.is_empty());
    }

    #[test]
    fn test_missing_key_error() {
        let kdl_input = r#"algorithm name="xxhash32""#;
        let doc: KdlDocument = kdl_input.parse().unwrap();

        let ctx = ParseContext::new(&doc, Current::Document(&doc), "test");
        let result = KeyProfileParser.parse(ctx);

        let msg_err = result.unwrap_err().help().unwrap().to_string();
        crate::assert_err_contains!(msg_err, "Missing required directive 'key'");
    }
}
