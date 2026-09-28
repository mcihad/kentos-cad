//! JSON-RPC 2.0 as MCP asks, both eras (docs/adr/0133):
//!
//! - modern (2026-07-28): every request carries its protocol version and the
//!   client's capabilities in `_meta`; no state is taken from earlier
//!   requests; results carry `resultType` and the server's identity;
//! - legacy (2025-11-25, 2025-06-18, 2025-03-26): an `initialize` request
//!   agrees on a version for this process, after which requests carry none.

use std::collections::BTreeMap;

use kentos_headless::Session;
use serde_json::{Value, json};

use crate::tools;

/// The protocol revision this server speaks per request.
pub const MODERN: &str = "2026-07-28";
/// The handshake revisions it answers `initialize` for, newest first.
pub const LEGACY: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];
pub const NAME: &str = "kentos-mcp";

const VERSION_KEY: &str = "/_meta/io.modelcontextprotocol~1protocolVersion";
const CAPABILITIES_KEY: &str = "/_meta/io.modelcontextprotocol~1clientCapabilities";

pub(crate) const INSTRUCTIONS: &str = "KentOS CAD çizimleri (.kcad) üzerinde çalışır. \
Önce drawing.open (bir dosya) ya da drawing.new (koordinat sistemiyle yeni proje) ile bir çizim açın; \
dönen `drawing` tutamacını sonraki her çağrıya verin. Komut araçları (cad.polygon.create …) kataloğun \
ürün komutlarıdır: op=plan yazılacak olanı gösterir ve hiçbir şey yazmaz, op=execute tek geri alma \
adımı olarak yazar. Nesneler kalıcı kimlikleriyle (uid) anılır. Koordinatlar x doğu (Y), y kuzey (X), \
projenin biriminde (m); alan m². Büyük çizimi drawing.entities ile sayfa sayfa, katman, tür ve kutu \
süzgeciyle okuyun. Değişiklikler drawing.save ile kaydedilene kadar yalnız bellektedir.";

/// A JSON-RPC error: its code, message and data.
pub(crate) struct Fault {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl Fault {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

/// The drawings open in this process, by handle; the handshake a legacy client made.
pub struct Server {
    pub(crate) drawings: BTreeMap<String, Session>,
    /// The KentOS server's account, from the environment (cloud.rs).
    pub(crate) account: crate::cloud::Account,
    /// The desktops attached, by handle (desktop.rs).
    pub(crate) desktops: BTreeMap<String, crate::desktop::Desktop>,
    legacy: Option<String>,
    pub(crate) catalog: Value,
    tools: Vec<Value>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let catalog = kentos_headless::catalog();
        let tools = tools::list(&catalog);
        Self {
            drawings: BTreeMap::new(),
            account: crate::cloud::Account::default(),
            desktops: BTreeMap::new(),
            legacy: None,
            catalog,
            tools,
        }
    }

    /// Answers one message; none for a notification.
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        if !message.is_object() {
            // Not a JSON-RPC message at all: said with a null id, as JSON-RPC asks.
            return Some(failure(
                &Value::Null,
                Fault::new(-32600, "Invalid Request: bir JSON-RPC nesnesi değil."),
            ));
        }
        let id = message.get("id").filter(|id| !id.is_null()).cloned();
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // Not a request: a stray response, or not JSON-RPC at all.
            return id.map(|id| failure(&id, Fault::new(-32600, "Invalid Request: method yok.")));
        };
        let Some(id) = id else {
            // Notifications (initialized, cancelled …) need no answer.
            return None;
        };
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let modern = match params.pointer(VERSION_KEY).and_then(Value::as_str) {
            Some(version) if version != MODERN => {
                return Some(failure(
                    &id,
                    Fault {
                        code: -32022,
                        message: "Unsupported protocol version".into(),
                        data: Some(json!({"supported": [MODERN], "requested": version})),
                    },
                ));
            }
            Some(_) if params.pointer(CAPABILITIES_KEY).is_none() => {
                return Some(failure(
                    &id,
                    Fault::new(
                        -32602,
                        "Invalid params: _meta io.modelcontextprotocol/clientCapabilities gerekli.",
                    ),
                ));
            }
            Some(_) => true,
            None => false,
        };
        if !modern {
            match method {
                "initialize" => return Some(self.initialize(&id, &params)),
                "ping" => return Some(success(&id, json!({}))),
                _ if self.legacy.is_some() => {}
                _ => {
                    return Some(failure(
                        &id,
                        Fault::new(
                            -32602,
                            format!(
                                "Invalid params: _meta io.modelcontextprotocol/protocolVersion gerekli ({MODERN}); \
                                 eski istemciler önce initialize göndermeli ({}).",
                                LEGACY.join(", ")
                            ),
                        ),
                    ));
                }
            }
        }
        let answer = match method {
            "server/discover" => Ok(self.discover()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.tools })),
            "tools/call" => self.call(&params),
            "resources/list" => Ok(json!({ "resources": tools::resources(self) })),
            "resources/read" => tools::read(self, &params),
            other => Err(Fault::new(-32601, format!("Method not found: {other}"))),
        };
        Some(match answer {
            Ok(mut result) => {
                if modern && let Some(fields) = result.as_object_mut() {
                    fields.insert("resultType".into(), json!("complete"));
                    fields.insert(
                        "_meta".into(),
                        json!({ "io.modelcontextprotocol/serverInfo": server_info() }),
                    );
                }
                success(&id, result)
            }
            Err(fault) => failure(&id, fault),
        })
    }

    fn initialize(&mut self, id: &Value, params: &Value) -> Value {
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let version = LEGACY
            .iter()
            .find(|v| **v == requested)
            .copied()
            .unwrap_or(LEGACY[0]);
        self.legacy = Some(version.to_owned());
        success(
            id,
            json!({
                "protocolVersion": version,
                "capabilities": capabilities(),
                "serverInfo": server_info(),
                "instructions": INSTRUCTIONS,
            }),
        )
    }

    fn discover(&self) -> Value {
        json!({
            "supportedVersions": [MODERN],
            "capabilities": capabilities(),
            "instructions": INSTRUCTIONS,
        })
    }

    fn call(&mut self, params: &Value) -> Result<Value, Fault> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| Fault::new(-32602, "Invalid params: name gerekli."))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if !arguments.is_object() {
            return Err(Fault::new(
                -32602,
                "Invalid params: arguments bir nesne olmalı.",
            ));
        }
        if !self.tools.iter().any(|t| t["name"] == name) {
            return Err(Fault::new(-32602, format!("Unknown tool: {name}")));
        }
        Ok(tools::call(self, name, &arguments))
    }
}

fn capabilities() -> Value {
    json!({ "tools": {}, "resources": {} })
}

fn server_info() -> Value {
    json!({ "name": NAME, "title": "KentOS CAD", "version": env!("CARGO_PKG_VERSION") })
}

fn success(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn failure(id: &Value, fault: Fault) -> Value {
    let mut error = json!({ "code": fault.code, "message": fault.message });
    if let Some(data) = fault.data {
        error["data"] = data;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": error })
}
