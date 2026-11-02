//! Dev control (feature `dev-control`): another process drives the app over
//! a loopback JSON RPC — click, type, keys, scroll, the accessibility tree,
//! screenshots and the app's own state report ([`crate::App::automation_state`]).
//!
//! An app turns it on at build time with the one feature and at run time with
//! [`crate::host::native::DevControlConfig`] in `NativeOptions` (the native
//! host starts the server). A build without the feature has none of this
//! code. The transport, the token and the discovery file are in
//! [`uzor_devtools`] (re-exported as [`devtools`]); the RPC core is
//! [`Dispatcher`].

mod keys;
mod protocol;

pub use protocol::{Dispatcher, RpcError, METHODS, PROTOCOL_VERSION};
pub use uzor_devtools as devtools;

use std::sync::Arc;

use crate::automation::AutomationClient;

/// A running dev-control endpoint: the loopback server and its discovery
/// file. Dropping it stops serving and removes the file.
pub struct DevControl {
    endpoint: uzor_devtools::Endpoint,
    dispatcher: Arc<Dispatcher>,
}

impl DevControl {
    /// The loopback port.
    pub fn port(&self) -> u16 {
        self.endpoint.port()
    }

    /// The discovery file (`$XDG_RUNTIME_DIR/<app_dir>/dev-<pid>.json`, mode 0600).
    pub fn discovery_path(&self) -> &std::path::Path {
        self.endpoint.discovery_path()
    }

    /// The RPC core (for another transport in the same process).
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }
}

/// Serve `client` on `127.0.0.1:<random port>` behind a fresh token and
/// publish the discovery file under `app_dir` (e.g. `"hatchery-hq"`); `app`
/// names the app in `ping`. The native host calls this itself for
/// `NativeOptions::dev_control`; call it directly for a headless host
/// ([`crate::HeadlessHost::serve_automation`]).
pub fn start(app_dir: &str, app: &str, client: AutomationClient) -> std::io::Result<DevControl> {
    let dispatcher = Arc::new(Dispatcher::new(app, client));
    let endpoint = uzor_devtools::serve(app_dir, app, Arc::clone(&dispatcher) as Arc<dyn uzor_devtools::Rpc>)?;
    Ok(DevControl { endpoint, dispatcher })
}
