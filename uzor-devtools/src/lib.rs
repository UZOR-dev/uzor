//! Dev-time remote control for uzor apps (see the crate README).
//!
//! The core ([`Dispatcher`]) is transport-agnostic: one JSON request string
//! in, one JSON response string out, plus an event log. [`server`] is a thin
//! loopback HTTP wrapper over it; [`start`] wires the server, a fresh
//! token and the discovery file for an app in one call.

#![deny(missing_docs)]

pub mod discovery;
pub mod keys;
pub mod protocol;
pub mod server;
pub mod token;

pub use discovery::{Discovery, DiscoveryInfo};
pub use protocol::{Dispatcher, PROTOCOL_VERSION};
pub use server::Server;
pub use token::Token;

use std::sync::Arc;

use uzor_framework::automation::AutomationClient;

/// A running dev-control endpoint: the server plus its discovery file.
/// Dropping it stops serving and removes the file.
pub struct DevControl {
    server: Server,
    discovery: Discovery,
    dispatcher: Arc<Dispatcher>,
}

impl DevControl {
    /// The loopback port.
    pub fn port(&self) -> u16 {
        self.server.port()
    }

    /// The discovery file path.
    pub fn discovery_path(&self) -> &std::path::Path {
        self.discovery.path()
    }

    /// The transport-agnostic core (for other transports in the same process).
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }
}

/// Start serving `client` on `127.0.0.1:<random port>` with a fresh token
/// and publish the discovery file under the app directory `app_dir`
/// (e.g. `"hatchery-hq"`). `app` names the app in `ping`.
pub fn start(app_dir: &str, app: &str, client: AutomationClient) -> std::io::Result<DevControl> {
    let token = Token::generate()?;
    let dispatcher = Arc::new(Dispatcher::new(app, client));
    let server = Server::start(Arc::clone(&dispatcher), token.clone())?;
    let info = DiscoveryInfo {
        app: app.to_string(),
        pid: std::process::id(),
        port: server.port(),
        token: token.to_hex(),
        protocol: PROTOCOL_VERSION,
    };
    let discovery = Discovery::publish(app_dir, &info)?;
    Ok(DevControl { server, discovery, dispatcher })
}
