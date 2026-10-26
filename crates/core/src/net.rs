//! Shared HTTP agent (Windows uses the OS TLS stack).

use std::time::Duration;

pub fn agent() -> ureq::Agent {
    let builder = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).user_agent(concat!("ApexOverlays/", env!("CARGO_PKG_VERSION")));
    #[cfg(windows)]
    let builder = match native_tls::TlsConnector::new() {
        Ok(c) => builder.tls_connector(std::sync::Arc::new(c)),
        Err(_) => builder,
    };
    builder.build()
}
