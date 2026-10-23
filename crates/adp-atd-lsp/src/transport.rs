//! The transports of the language server (research R10): an in-memory connection on a worker thread for the
//! desktop host, stdio for IntelliJ's `atd-lsp`, and a WebAssembly interface for VS Code. Each runs the same
//! [`Server`]; diagnostics follow a change by 150 ms of quiet.

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

/// How long after the last change diagnostics are published.
pub const DEBOUNCE_MS: u64 = 150;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::time::Duration;

    use lsp_server::{Connection, Message};
    use serde_json::Value as Json;

    use super::DEBOUNCE_MS;
    use crate::Server;

    fn to_json(message: &Message) -> Json {
        serde_json::to_value(message).unwrap_or(Json::Null)
    }

    fn from_json(value: Json) -> Option<Message> {
        serde_json::from_value(value).ok()
    }

    /// Serves one connection until the client exits: messages in order, diagnostics after 150 ms of quiet.
    pub fn serve(connection: &Connection, server: &mut Server) {
        loop {
            let message = if server.has_pending() {
                match connection.receiver.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                    Ok(message) => Some(message),
                    Err(crossbeam_timeout) if crossbeam_timeout.is_timeout() => None,
                    Err(_) => return,
                }
            } else {
                match connection.receiver.recv() {
                    Ok(message) => Some(message),
                    Err(_) => return,
                }
            };
            let outgoing = match message {
                None => server.flush(),
                Some(Message::Notification(n)) if n.method == "exit" => return,
                Some(message) => server.handle(&to_json(&message)),
            };
            for value in outgoing {
                if let Some(message) = from_json(value)
                    && connection.sender.send(message).is_err()
                {
                    return;
                }
            }
        }
    }

    /// The server on a worker thread of this process, over an in-memory connection; no socket is opened (the
    /// desktop host's principle I). The returned connection is the client's end.
    pub fn spawn_in_memory() -> (Connection, std::thread::JoinHandle<()>) {
        let (server_end, client_end) = Connection::memory();
        let handle = std::thread::Builder::new()
            .name("atd-lsp".into())
            .spawn(move || {
                let mut server = Server::new();
                serve(&server_end, &mut server);
            })
            .expect("the language server's thread starts");
        (client_end, handle)
    }

    /// The server over stdin and stdout, as `atd-lsp` runs it.
    pub fn run_stdio() {
        let (connection, io_threads) = Connection::stdio();
        let mut server = Server::new();
        serve(&connection, &mut server);
        drop(connection);
        let _ = io_threads.join();
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use serde_json::Value as Json;
    use wasm_bindgen::prelude::wasm_bindgen;

    use crate::Server;

    /// The server for a JavaScript host: messages in as JSON text, the messages to send out as a JSON array.
    #[wasm_bindgen]
    pub struct WasmServer {
        server: Server,
    }

    #[wasm_bindgen]
    impl WasmServer {
        #[wasm_bindgen(constructor)]
        pub fn new() -> WasmServer {
            WasmServer {
                server: Server::with_files(Box::new(NoFiles)),
            }
        }

        /// Handles one message; gives the messages to send, as a JSON array.
        pub fn handle(&mut self, message: &str) -> String {
            let value: Json = serde_json::from_str(message).unwrap_or(Json::Null);
            Json::Array(self.server.handle(&value)).to_string()
        }

        /// Whether a change waits for its diagnostics.
        pub fn pending(&self) -> bool {
            self.server.has_pending()
        }

        /// The diagnostics of the changes since the last call, as a JSON array of messages.
        pub fn flush(&mut self) -> String {
            Json::Array(self.server.flush()).to_string()
        }
    }

    impl Default for WasmServer {
        fn default() -> Self {
            WasmServer::new()
        }
    }

    /// WebAssembly reads no file: every file of a description is a document the client opened.
    struct NoFiles;

    impl adp_atd_check::Files for NoFiles {
        fn read(&self, _: &std::path::Path) -> Option<String> {
            None
        }

        fn list(&self, _: &std::path::Path) -> Vec<std::path::PathBuf> {
            Vec::new()
        }
    }
}
