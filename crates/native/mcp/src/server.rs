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
/// What every message of a `subscriptions/listen` stream carries in `_meta`: the listen request's id.
const SUBSCRIPTION: &str = "io.modelcontextprotocol/subscriptionId";

pub(crate) const INSTRUCTIONS: &str = "KentOS CAD çizimleri (.kcad) üzerinde çalışır. \
Önce drawing.open (bir dosya) ya da drawing.new (koordinat sistemiyle yeni proje) ile bir çizim açın; \
dönen `drawing` tutamacını sonraki her çağrıya verin. Komut araçları (cad.polygon.create …) kataloğun \
ürün komutlarıdır: op=plan yazılacak olanı gösterir ve hiçbir şey yazmaz, op=execute tek geri alma \
adımı olarak yazar. Nesneler kalıcı kimlikleriyle (uid) anılır. Koordinatlar x doğu (Y), y kuzey (X), \
projenin biriminde (m); alan m². Her sayının birimi araç şemasının x-unit'idir (açılar alanına göre derece ya da \
radyan); drawing.summary'nin conventions'ı arayüzün eksen adlarını ve kutupsal açının yönünü söyler. Büyük çizimi \
drawing.entities ile sayfa sayfa, katman, tür ve kutu süzgeciyle okuyun. Değişiklikler drawing.save ile \
kaydedilene kadar yalnız bellektedir.";

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

/// Which tools the server offers (docs/adr/8056 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// The v2 surface, what the program starts with: the server's own tools for drawings, desktops and the cloud,
    /// and the agents' tools (the catalog step by step, a door per part, a write planned first:
    /// `kentos_headless::agent_tools`).
    Compact,
    /// Every catalog command a tool of its own, written at once, as before docs/adr/8056 §2: `--profile full`, and
    /// what [`Server::new`] gives the tests and earlier hosts.
    Full,
}

impl Profile {
    /// The program's choice: `--profile compact|full`, else `KENTOS_MCP_PROFILE`, else compact.
    pub fn chosen(args: &[String], env: Option<&str>) -> Result<Self, String> {
        let named = args
            .iter()
            .position(|a| a == "--profile")
            .map(|i| args.get(i + 1).map(String::as_str).unwrap_or_default())
            .or(env);
        match named {
            None | Some("compact") => Ok(Profile::Compact),
            Some("full") => Ok(Profile::Full),
            Some(other) => Err(format!("profil “{other}” olamaz: compact ya da full.")),
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
    pub(crate) profile: Profile,
    /// The plans each drawing's agent was shown (the owner's guard, docs/adr/8056 §2), by handle.
    pub(crate) guards: BTreeMap<String, kentos_headless::agent_tools::Guard>,
    /// The open `subscriptions/listen` streams watching `kentos://extensions/installed` (2026-07-28), by their
    /// request's id.
    streams: Vec<Value>,
    /// A legacy client's handshake is over (`notifications/initialized`): notices may go to it.
    legacy_ready: bool,
    /// A legacy client subscribed to `kentos://extensions/installed` (`resources/subscribe`).
    legacy_watch: bool,
    /// The installed extensions as last told, while anyone listens.
    watch: Option<crate::extensions::Watch>,
    /// The user's extension folder (`kentos://extensions/installed`); none when this computer gives no data folder.
    pub(crate) extensions_root: Option<std::path::PathBuf>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// The server with every catalog command a tool of its own ([`Profile::Full`]), as the tests and earlier hosts
    /// expect; the program starts [`Server::with_profile`]'s compact one.
    pub fn new() -> Self {
        Self::with_profile(Profile::Full)
    }

    pub fn with_profile(profile: Profile) -> Self {
        // Bound layers on PostGIS (docs/adr/0505): the provider, once per process.
        kentos_postgis::register();
        // STAC searches (docs/adr/0528 §2): over the cloud client's HTTP; CBS's (docs/adr/8054 §11).
        #[cfg(feature = "mod-gis")]
        kentos_native_application::stac_host::register(Box::new(kentos_cloud::stac::HttpFetch));
        kentos_native_application::remote_host::register(Box::new(kentos_cloud::range::HttpRemote));
        let catalog = kentos_headless::catalog();
        let tools = match profile {
            Profile::Full => tools::list(&catalog),
            Profile::Compact => tools::compact(&catalog),
        };
        Self {
            drawings: BTreeMap::new(),
            account: crate::cloud::Account::default(),
            desktops: BTreeMap::new(),
            legacy: None,
            catalog,
            tools,
            profile,
            guards: BTreeMap::new(),
            streams: Vec::new(),
            legacy_ready: false,
            legacy_watch: false,
            watch: None,
            extensions_root: kentos_extension_host::places::user_root(),
        }
    }

    /// The server reading the extensions installed in `root` instead of the user's (a test's registry, a host's).
    pub fn with_extensions_root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.extensions_root = Some(root.into());
        self.watch = None;
        self
    }

    /// The notifications due now; the program asks after every message and every two seconds between them. When the
    /// installed extensions read differently (one installed, enabled, disabled, updated, removed, failed, or IT's
    /// policy changed), each stream and legacy client watching `kentos://extensions/installed` hears
    /// `notifications/resources/updated` for it.
    pub fn notices(&mut self) -> Vec<Value> {
        // Watched while anyone subscribed; a legacy client hears only once its handshake is over.
        if self.streams.is_empty() && !self.legacy_watch {
            self.watch = None;
            return Vec::new();
        }
        let watch = self
            .watch
            .get_or_insert_with(|| crate::extensions::Watch::new(self.extensions_root.clone()));
        if !watch.changed() {
            return Vec::new();
        }
        let uri = crate::extensions::INSTALLED;
        let mut out: Vec<Value> = self
            .streams
            .iter()
            .map(|id| {
                json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/resources/updated",
                    "params": { "_meta": { SUBSCRIPTION: id }, "uri": uri },
                })
            })
            .collect();
        if self.legacy_watch && self.legacy_ready {
            out.push(json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/updated",
                "params": { "uri": uri },
            }));
        }
        out
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
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(id) = id else {
            // Notifications need no answer. The legacy handshake's end lets notices go; a cancellation ends the stream
            // whose request it names.
            match method {
                "notifications/initialized" => self.legacy_ready = true,
                "notifications/cancelled" => {
                    if let Some(request) = params.get("requestId") {
                        self.streams.retain(|open| open != request);
                    }
                }
                _ => {}
            }
            return None;
        };
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
            // The stream's first message is its acknowledgement, not an answer: the request stays open.
            "subscriptions/listen" if modern => return Some(self.listen(&id, &params)),
            // 2025-11-25 and before; 2026-07-28 asks through subscriptions/listen.
            "resources/subscribe" if !modern => self.subscribe(true, &params),
            "resources/unsubscribe" if !modern => self.subscribe(false, &params),
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
                    if let Some((ttl, scope)) = cache_hint(method, &params) {
                        fields.insert("ttlMs".into(), json!(ttl));
                        fields.insert("cacheScope".into(), json!(scope));
                    }
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

    /// `subscriptions/listen` (2026-07-28): of what the client asks, updates of `kentos://extensions/installed` are
    /// honoured; the acknowledgement says which, and is the stream's first message. The stream stays open until the
    /// client cancels it (`notifications/cancelled`) or the process ends; the tool list does not change with the
    /// extensions yet (their commands are Python's, ADIM 19), so `toolsListChanged` is not taken.
    fn listen(&mut self, id: &Value, params: &Value) -> Value {
        let uri = crate::extensions::INSTALLED;
        let watched = params
            .pointer("/notifications/resourceSubscriptions")
            .and_then(Value::as_array)
            .is_some_and(|uris| uris.iter().any(|u| u == uri));
        let mut agreed = serde_json::Map::new();
        if watched {
            agreed.insert("resourceSubscriptions".into(), json!([uri]));
            if !self.streams.contains(id) {
                self.streams.push(id.clone());
            }
            self.watch
                .get_or_insert_with(|| crate::extensions::Watch::new(self.extensions_root.clone()));
        }
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/subscriptions/acknowledged",
            "params": { "_meta": { SUBSCRIPTION: id }, "notifications": agreed },
        })
    }

    /// `resources/subscribe` and `resources/unsubscribe` (2025-11-25 and before): `kentos://extensions/installed` is the
    /// resource watched; another is refused, saying which is.
    fn subscribe(&mut self, on: bool, params: &Value) -> Result<Value, Fault> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| Fault::new(-32602, "Invalid params: uri gerekli."))?;
        if uri != crate::extensions::INSTALLED {
            return Err(Fault::new(
                -32602,
                format!(
                    "Invalid params: {uri} için güncelleme bildirimi yok; izlenebilen kaynak {}.",
                    crate::extensions::INSTALLED
                ),
            ));
        }
        self.legacy_watch = on;
        if on {
            self.watch
                .get_or_insert_with(|| crate::extensions::Watch::new(self.extensions_root.clone()));
        }
        Ok(json!({}))
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

/// Resources can be watched (`kentos://extensions/installed`); the tool list is the build's and does not change.
fn capabilities() -> Value {
    json!({ "tools": {}, "resources": { "subscribe": true } })
}

/// The caching hints 2026-07-28 asks of these results (`ttlMs`, `cacheScope`): what is the build's (the capabilities,
/// the tools, the catalog) is fresh for an hour and the same for anyone; what holds the user's drawings or extensions
/// is stale at once and theirs.
fn cache_hint(method: &str, params: &Value) -> Option<(u64, &'static str)> {
    const HOUR: u64 = 3_600_000;
    match method {
        "server/discover" | "tools/list" => Some((HOUR, "public")),
        "resources/list" => Some((0, "private")),
        "resources/read" => match params.get("uri").and_then(Value::as_str) {
            Some("kentos://catalog" | "kentos://catalog/categories") => Some((HOUR, "public")),
            _ => Some((0, "private")),
        },
        _ => None,
    }
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
