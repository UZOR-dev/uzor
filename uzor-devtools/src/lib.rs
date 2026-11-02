//! The transport side of uzor's dev control (see the crate README): a
//! loopback HTTP server with token access checks, the discovery file, the
//! event log and the `uzor-dev` CLI.
//!
//! This crate does not know the framework. The RPC core lives in
//! `uzor-framework` (feature `dev-control`, module `dev_control`) and plugs in
//! through [`Rpc`]; apps turn it on with that one feature.

#![deny(missing_docs)]

pub mod discovery;
pub mod events;
pub mod server;
pub mod token;

pub use discovery::{Discovery, DiscoveryInfo};
pub use server::Server;
pub use token::Token;

use std::sync::Arc;
use std::time::Duration;

/// Protocol revision reported by `ping` and the discovery file.
pub const PROTOCOL_VERSION: u32 = 1;

/// The methods of protocol 1 (answered by `uzor_framework::dev_control::Dispatcher`).
pub const METHODS: &[&str] = &[
    "ping", "windows", "dump-tree", "click", "hover", "focus", "type", "key", "scroll", "screenshot", "state",
    "wait-for", "settle", "record", "replay",
];

/// A transport-agnostic RPC core: one JSON request in, one JSON response
/// out, plus the event log a transport streams.
pub trait Rpc: Send + Sync + 'static {
    /// Answer one request (never panics on input).
    fn handle_json(&self, request: &str) -> String;
    /// Events with `seq > since`, waiting up to `wait` for the first one.
    fn events_since(&self, since: u64, wait: Duration) -> Vec<serde_json::Value>;
}

/// A running endpoint: the server plus its discovery file. Dropping it
/// stops serving and removes the file.
pub struct Endpoint {
    server: Server,
    discovery: Discovery,
}

impl Endpoint {
    /// The loopback port.
    pub fn port(&self) -> u16 {
        self.server.port()
    }

    /// The discovery file path.
    pub fn discovery_path(&self) -> &std::path::Path {
        self.discovery.path()
    }
}

/// Serve `rpc` on `127.0.0.1:<random port>` behind a fresh 32-byte token
/// and publish the discovery file under the app directory `app_dir`
/// (e.g. `"hatchery-hq"`). `app` names the app in the file.
pub fn serve(app_dir: &str, app: &str, rpc: Arc<dyn Rpc>) -> std::io::Result<Endpoint> {
    let token = Token::generate()?;
    let server = Server::start(rpc, token.clone())?;
    let info = DiscoveryInfo {
        app: app.to_string(),
        pid: std::process::id(),
        port: server.port(),
        token: token.to_hex(),
        protocol: PROTOCOL_VERSION,
    };
    let discovery = Discovery::publish(app_dir, &info)?;
    Ok(Endpoint { server, discovery })
}
