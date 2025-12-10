use std::net::SocketAddr;

use motya_macro::validate;

use crate::{
    common_types::{
        listeners::{ListenerConfig, ListenerKind, Listeners, TlsConfig},
        section_parser::SectionParser,
    },
    kdl::parser::{
        ctx::ParseContext,
        ensures::{NamePredicate, Rule},
        utils::{OptionTypedValueExt, PrimitiveType},
    },
};

pub struct ListenersSection;

impl SectionParser<ParseContext<'_>, Listeners> for ListenersSection {
    #[validate(ensure_node_name = "listeners")]
    fn parse_node(&self, ctx: ParseContext<'_>) -> miette::Result<Listeners> {
        let nodes = ctx.req_nodes()?;

        let list_cfgs = nodes
            .into_iter()
            .map(|node_ctx| self.extract_listener(node_ctx))
            .collect::<miette::Result<Vec<_>>>()?;

        Ok(Listeners { list_cfgs })
    }
}

impl ListenersSection {
    fn extract_listener(&self, ctx: ParseContext<'_>) -> miette::Result<ListenerConfig> {
        ctx.validate(&[
            Rule::NoChildren,
            Rule::NoPositionalArgs,
            Rule::OnlyKeysTyped(&[
                ("cert-path", PrimitiveType::String),
                ("key-path", PrimitiveType::String),
                ("offer-h2", PrimitiveType::Bool),
            ]),
            Rule::Name(NamePredicate::SocketAddr),
        ])?;

        let addr = ctx.validated_name()?.as_socket_addr()?;

        let [cert_opt, key_opt, h2_opt] = ctx.props(["cert-path", "key-path", "offer-h2"])?;

        self.resolve_tcp_listener(
            &ctx,
            addr,
            cert_opt.as_str()?,
            key_opt.as_str()?,
            h2_opt.as_bool()?,
        )
    }

    fn resolve_tcp_listener(
        &self,
        ctx: &ParseContext<'_>,
        addr: SocketAddr,
        cert_path: Option<String>,
        key_path: Option<String>,
        offer_h2: Option<bool>,
    ) -> miette::Result<ListenerConfig> {
        match (cert_path, key_path, offer_h2) {

            (None, None, None) => Ok(ListenerConfig {
                source: ListenerKind::Tcp {
                    addr: addr.to_string(),
                    tls: None,
                    offer_h2: false,
                },
            }),

            (None, Some(_), _) | (Some(_), None, _) => Err(ctx.error(
                "'cert-path' and 'key-path' must either BOTH be present, or NEITHER should be present",
            )),

            (None, None, Some(_)) => Err(ctx.error(
                "'offer-h2' requires TLS, specify 'cert-path' and 'key-path'",
            )),

            (Some(cpath), Some(kpath), offer_h2) => Ok(ListenerConfig {
                source: ListenerKind::Tcp {
                    addr: addr.to_string(),
                    tls: Some(TlsConfig {
                        cert_path: cpath.into(),
                        key_path: kpath.into(),
                    }),

                    offer_h2: offer_h2.unwrap_or(true),
                },
            }),
        }
    }
}
