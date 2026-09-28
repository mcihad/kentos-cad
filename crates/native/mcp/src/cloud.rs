//! The server's commands (docs/adr/0134): `project.*` of the catalog, the
//! account's project list and a project opened as a local drawing, through
//! the KentOS server with the account the environment names (MCP's rule for
//! stdio: credentials come from the environment, never from the model).
//!
//!   KENTOS_URL=http://127.0.0.1:8787 KENTOS_LOGIN=ayse KENTOS_PASSWORD=… kentos-mcp
//!
//! The server checks the account's rights on every command; nothing here
//! decides them. Requests say they come from `mcp` (`x-kentos-client`).

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};

use kentos_cloud::{ApiFailure, CatalogQuery, Cloud};
use kentos_contracts::{CatalogView, CommandEnvelope};
use kentos_domain::Uuid;
use serde_json::{Value, json};

/// The account and its session, made at the first server tool.
#[derive(Default)]
pub struct Account {
    cloud: Option<Cloud>,
}

/// Waits for a future on this thread (the cloud client runs its work on its
/// own runtime, so any waiter will do).
fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}

/// A failure of the server as a tool's refusal: its status, code, message
/// and the field or revision it is about.
pub fn refusal(f: &ApiFailure) -> Value {
    let mut error = json!({ "status": f.status, "code": f.code, "message": f.message, "retryable": f.retryable });
    if let Some(path) = &f.path {
        error["path"] = json!(path);
    }
    if let Some(revision) = &f.revision {
        error["revision"] = json!(revision);
    }
    let value = json!({ "error": error });
    json!({
        "content": [{ "type": "text", "text": format!("{} {}: {}", f.status, f.code, f.message) }],
        "structuredContent": value,
        "isError": true,
    })
}

fn not_configured() -> ApiFailure {
    ApiFailure::new(
        0,
        "not_configured",
        "Sunucu komutları için MCP sunucusunun ortamında KENTOS_URL, KENTOS_LOGIN ve KENTOS_PASSWORD \
         gerekli (istemcinin sunucu tanımında verilir; model vermez).",
    )
}

impl Account {
    /// The signed-in connection, signing in at the first need.
    fn cloud(&mut self) -> Result<Cloud, ApiFailure> {
        if let Some(cloud) = &self.cloud
            && cloud.signed_in()
        {
            return Ok(cloud.clone());
        }
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());
        let (Some(url), Some(login), Some(password)) = (
            var("KENTOS_URL"),
            var("KENTOS_LOGIN"),
            var("KENTOS_PASSWORD"),
        ) else {
            return Err(not_configured());
        };
        let cloud = Cloud::for_program(&url, "mcp")?;
        block_on(cloud.sign_in(&login, &password))?;
        self.cloud = Some(cloud.clone());
        Ok(cloud)
    }

    /// Runs `work` signed in; a session the server ended is made again once.
    fn with<T>(&mut self, work: impl Fn(&Cloud) -> Result<T, ApiFailure>) -> Result<T, ApiFailure> {
        let cloud = self.cloud()?;
        match work(&cloud) {
            Err(f) if f.status == 401 => {
                self.cloud = None;
                let cloud = self.cloud()?;
                work(&cloud)
            }
            other => other,
        }
    }

    /// A page of the account's projects (`view`: mine, organization, shared,
    /// recent, favorites, archived, trash).
    pub fn projects(&mut self, args: &Value) -> Result<Value, ApiFailure> {
        let view: CatalogView = serde_json::from_value(json!(
            args.get("view").and_then(Value::as_str).unwrap_or("mine")
        ))
        .map_err(|_| {
            ApiFailure::new(
                0,
                "invalid_input",
                "view: mine, organization, shared, recent, favorites, archived ya da trash.",
            )
        })?;
        let mut query = CatalogQuery::new(view);
        query.text = args.get("q").and_then(Value::as_str).map(str::to_owned);
        query.limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .and_then(|l| u32::try_from(l.clamp(1, 200)).ok());
        query.after = args.get("after").and_then(Value::as_str).map(str::to_owned);
        query.tenant = match args.get("tenant").and_then(Value::as_str) {
            Some(t) => Some(uuid(t, "tenant")?),
            None => None,
        };
        let page = self.with(|c| block_on(c.catalog(&query)))?;
        serde_json::to_value(page).map_err(|e| ApiFailure::new(0, "answer", e.to_string()))
    }

    /// A database project's `.kcad` of this moment.
    pub fn snapshot(
        &mut self,
        tenant: &str,
        project: &str,
    ) -> Result<(Vec<u8>, Option<String>), ApiFailure> {
        let (tenant, project) = (uuid(tenant, "tenant")?, uuid(project, "project")?);
        let download = self.with(|c| block_on(c.snapshot(tenant, project, None)))?;
        Ok((download.bytes, download.revision))
    }

    /// Runs a catalog command on the server: its output as the server answers it.
    pub fn command(
        &mut self,
        id: &str,
        version: u32,
        scope: &str,
        args: &Value,
    ) -> Result<Value, ApiFailure> {
        let tenant = uuid(text(args, "tenant")?, "tenant")?;
        let project = if scope == "tenant" {
            String::new()
        } else {
            uuid(text(args, "project")?, "project")?.to_string()
        };
        let mut input = args.clone();
        if let Some(fields) = input.as_object_mut() {
            for own in ["tenant", "project", "expectedVersions", "idempotencyKey"] {
                fields.remove(own);
            }
        }
        let expected = match args.get("expectedVersions") {
            None | Some(Value::Null) => Default::default(),
            Some(v) => serde_json::from_value(v.clone()).map_err(|_| {
                ApiFailure::new(
                    0,
                    "invalid_input",
                    "expectedVersions metin değerli bir nesne olmalı: {\"@project\": \"7\"}.",
                )
            })?,
        };
        let key = args
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
        let envelope = CommandEnvelope {
            command_name: id.to_owned(),
            version,
            tenant_id: tenant.to_string(),
            project_id: project,
            request_id: format!("mcp-{}", Uuid::new_v4()),
            idempotency_key: key,
            expected_versions: expected,
            input,
        };
        self.with(|c| {
            if scope == "tenant" {
                block_on(c.tenant_command::<Value>(envelope.clone()))
            } else {
                block_on(c.command::<Value>(envelope.clone()))
            }
        })
    }
}

fn text<'a>(args: &'a Value, name: &str) -> Result<&'a str, ApiFailure> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiFailure::new(0, "invalid_input", format!("{name} gerekli.")))
}

fn uuid(value: &str, name: &str) -> Result<Uuid, ApiFailure> {
    Uuid::parse_str(value).map_err(|_| {
        ApiFailure::new(
            0,
            "invalid_input",
            format!("{name} bir UUID olmalı (project.list verir)."),
        )
    })
}
