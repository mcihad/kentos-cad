//! Veritabanı bağlantıları (docs/adr/0623): the device's PostgreSQL connections, kept as QGIS keeps its own. A
//! connection is a name, the server, its port, the database, the user, how TLS is asked for and, only when the user
//! ticked Parolayı sakla, the password. Nobody types a connection string: the address a bound layer keeps
//! ([`DbConnection::address`]) is built from these fields and carries no password.
//!
//! The connections live in `$XDG_CONFIG_HOME/kentos-cad/veritabani-baglantilari.json` (else `~/.config/kentos-cad/`),
//! readable by the user alone (0600), as the map services' secrets are (docs/adr/0208 §12); a file that does not read
//! is set aside, never written over. A password typed for this session alone is kept in memory
//! ([`remember`]). When a command reaches a database, its password is looked up by the server, port, database and
//! user ([`password_for`]): the session's, then the device's, then `PGPASSWORD`, then `~/.pgpass`; so the drawing
//! never holds one, and Python and the MCP server on the same computer reach what the desktop saved.
//!
//! Dışa aktar and İçe aktar give a team the same connections ([`export`], [`import`]): the device file's own format,
//! the passwords only when asked.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use serde::{Deserialize, Serialize};

/// PostgreSQL's port when none is said.
pub const DEFAULT_PORT: u16 = 5432;
const FILE: &str = "veritabani-baglantilari.json";
const FORMAT: &str = "kentos.db-connections";

/// How TLS is asked for (libpq's `sslmode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SslMode {
    Disable,
    Allow,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

/// The modes as the connection window lists them: the usual first.
pub const SSL_MODES: [SslMode; 6] = [
    SslMode::Prefer,
    SslMode::Require,
    SslMode::VerifyFull,
    SslMode::VerifyCa,
    SslMode::Allow,
    SslMode::Disable,
];

impl SslMode {
    /// libpq's name, as an address carries it.
    pub fn libpq(self) -> &'static str {
        match self {
            SslMode::Disable => "disable",
            SslMode::Allow => "allow",
            SslMode::Prefer => "prefer",
            SslMode::Require => "require",
            SslMode::VerifyCa => "verify-ca",
            SslMode::VerifyFull => "verify-full",
        }
    }

    pub fn from_libpq(text: &str) -> Option<SslMode> {
        SSL_MODES.into_iter().find(|m| m.libpq().eq_ignore_ascii_case(text.trim()))
    }

    /// Its name in the window.
    pub fn label(self) -> &'static str {
        match self {
            SslMode::Prefer => "Varsa kullan",
            SslMode::Require => "Zorunlu",
            SslMode::VerifyFull => "Zorunlu, sertifika ve sunucu adı doğrulanır",
            SslMode::VerifyCa => "Zorunlu, sertifika doğrulanır",
            SslMode::Allow => "Sunucu isterse",
            SslMode::Disable => "Kapalı",
        }
    }

    /// What it means, for the window's note.
    pub fn note(self) -> &'static str {
        match self {
            SslMode::Prefer => "Sunucu destekliyorsa bağlantı şifrelenir, desteklemiyorsa şifresiz kurulur. Çoğu kurum için uygundur.",
            SslMode::Require => "Bağlantı her zaman şifrelenir; sunucu şifrelemeyi desteklemiyorsa bağlanılmaz. Sertifika doğrulanmaz.",
            SslMode::VerifyFull => "Şifreli bağlantı; sunucunun sertifikası ve adı sistemin güvendiği sertifikalarla doğrulanır. En güvenlisidir.",
            SslMode::VerifyCa => "Şifreli bağlantı; sunucunun sertifikası doğrulanır, adı doğrulanmaz.",
            SslMode::Allow => "Önce şifresiz denenir; sunucu isterse şifreli kurulur.",
            SslMode::Disable => "Bağlantı şifrelenmez. Yalnız kendi bilgisayarınızdaki ya da güvendiğiniz ağdaki sunucu için seçin.",
        }
    }
}

/// A connection to a PostgreSQL database.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbConnection {
    /// Its name in the lists (Belediye, Kadastro sunucusu); unique on the device.
    pub name: String,
    /// The server's name or IP address.
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database: String,
    /// The user's name; empty: libpq's default (`PGUSER`, else the computer's user).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(default)]
    pub ssl: SslMode,
    /// Kept only when the user ticked Parolayı sakla.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Only the `public` schema is listed (QGIS's “Only look in the 'public' schema”).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub public_only: bool,
    /// Tables without geometry are listed too (QGIS's “Also list tables with no geometry”).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub geometryless: bool,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for DbConnection {
    fn default() -> Self {
        DbConnection {
            name: String::new(),
            host: String::new(),
            port: DEFAULT_PORT,
            database: String::new(),
            user: String::new(),
            ssl: SslMode::Prefer,
            password: None,
            public_only: false,
            geometryless: false,
        }
    }
}

/// A field of the connection window, for where a problem is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Host,
    Port,
    Database,
    User,
}

impl DbConnection {
    /// The address a bound layer keeps (`postgresql://kullanıcı@sunucu:5432/veritabanı`): no password ever, the
    /// user and the database percent-encoded, the port always, the TLS mode when it is not the default.
    pub fn address(&self) -> String {
        let mut a = String::from("postgresql://");
        if !self.user.is_empty() {
            a.push_str(&encode(&self.user));
            a.push('@');
        }
        let host = self.host.trim();
        if host.contains(':') && !host.starts_with('[') {
            a.push('[');
            a.push_str(host);
            a.push(']');
        } else {
            a.push_str(host);
        }
        a.push(':');
        a.push_str(&self.port.to_string());
        a.push('/');
        a.push_str(&encode(self.database.trim()));
        if self.ssl != SslMode::Prefer {
            a.push_str("?sslmode=");
            a.push_str(self.ssl.libpq());
        }
        a
    }

    /// What it reaches, said short: `ali@db.belediye.local:5432/kent`.
    pub fn summary(&self) -> String {
        let user = if self.user.is_empty() {
            String::new()
        } else {
            format!("{}@", self.user)
        };
        format!("{user}{}:{}/{}", self.host.trim(), self.port, self.database.trim())
    }

    /// What is wrong with it, field by field, before it is saved or tried.
    pub fn problems(&self) -> Vec<(Field, String)> {
        let mut out = Vec::new();
        if self.name.trim().is_empty() {
            out.push((Field::Name, "Bağlantıya bir ad verin; listelerde bu adla görünür.".to_owned()));
        }
        let host = self.host.trim();
        if host.is_empty() {
            out.push((
                Field::Host,
                "Sunucunun adını ya da IP adresini yazın (örneğin db.belediye.local ya da 10.0.0.5).".to_owned(),
            ));
        } else if host.contains(char::is_whitespace) || host.contains('/') || host.contains('@') {
            out.push((
                Field::Host,
                "Sunucu adı boşluk, / ya da @ içermez; yalnız adı ya da IP adresini yazın.".to_owned(),
            ));
        }
        if self.port == 0 {
            out.push((
                Field::Port,
                "Bağlantı noktası 1 ile 65535 arasında bir sayıdır; PostgreSQL'de genellikle 5432'dir.".to_owned(),
            ));
        }
        if self.database.trim().is_empty() {
            out.push((Field::Database, "Veritabanının adını yazın.".to_owned()));
        }
        if self.user.contains(char::is_whitespace) {
            out.push((Field::User, "Kullanıcı adı boşluk içermez.".to_owned()));
        }
        out
    }

    /// Whether it reaches `key`'s database as `key`'s user (an empty user: any).
    fn reaches(&self, key: &Key) -> bool {
        self.host.trim().eq_ignore_ascii_case(&key.host)
            && self.port == key.port
            && self.database.trim() == key.database
            && (self.user.is_empty() || self.user == key.user)
    }
}

/// An address's or a connection string's fields, as pasted (Adresten doldur): a `postgresql://` address or libpq's
/// `anahtar=değer` words (`host=… dbname=… user=…`). The password, when the text carries one, is given apart: it is
/// never kept in an address.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub connection: DbConnection,
    pub password: Option<String>,
}

/// Reads a pasted address or connection string; why not, when it is neither.
pub fn parse(text: &str) -> Result<Parsed, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("Yapıştırılan metin boş.".to_owned());
    }
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("postgresql://") || lower.starts_with("postgres://") {
        return parse_url(t);
    }
    if t.contains('=') {
        return parse_words(t);
    }
    Err("Metin bir PostgreSQL adresi (postgresql://…) ya da bağlantı dizesi (host=… dbname=…) değil.".to_owned())
}

fn parse_url(text: &str) -> Result<Parsed, String> {
    let rest = text.split_once("://").map_or("", |(_, r)| r);
    let (main, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (authority, path) = main.split_once('/').unwrap_or((main, ""));
    let mut out = Parsed::default();
    let c = &mut out.connection;
    let host_port = match authority.rsplit_once('@') {
        Some((who, hp)) => {
            let (user, password) = who.split_once(':').unwrap_or((who, ""));
            c.user = decode(user)?;
            if !password.is_empty() {
                out.password = Some(decode(password)?);
            }
            hp
        }
        None => authority,
    };
    let (host, port) = if let Some(inner) = host_port.strip_prefix('[') {
        let (h, after) = inner.split_once(']').ok_or("Adresin IPv6 sunucusu ] ile kapanmıyor.")?;
        (h.to_owned(), after.strip_prefix(':').unwrap_or(""))
    } else {
        match host_port.rsplit_once(':') {
            Some((h, p)) => (h.to_owned(), p),
            None => (host_port.to_owned(), ""),
        }
    };
    c.host = decode(&host)?;
    if !port.is_empty() {
        c.port = port
            .parse::<u16>()
            .ok()
            .filter(|&p| p > 0)
            .ok_or_else(|| format!("“{port}” bir bağlantı noktası değil."))?;
    }
    c.database = decode(path)?;
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v)?;
        word(&mut out, k, v)?;
    }
    Ok(out)
}

fn parse_words(text: &str) -> Result<Parsed, String> {
    let mut out = Parsed::default();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        let start = i;
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() {
            i += 1;
        }
        let key: String = chars[start..i].iter().collect();
        if i >= chars.len() || chars[i] != '=' {
            // A word without a value (a table's `(geom)` after it): passed over.
            continue;
        }
        i += 1;
        let mut value = String::new();
        if i < chars.len() && (chars[i] == '\'' || chars[i] == '"') {
            let quote = chars[i];
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err(format!("“{key}” değerinin tırnağı kapanmıyor.")),
                    Some('\\') if chars.get(i + 1).is_some() => {
                        value.push(chars[i + 1]);
                        i += 2;
                    }
                    Some(&c) if c == quote => {
                        i += 1;
                        break;
                    }
                    Some(&c) => {
                        value.push(c);
                        i += 1;
                    }
                }
            }
            // `table="public"."parsel"`: the rest of the word.
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
        } else {
            while i < chars.len() && !chars[i].is_whitespace() {
                value.push(chars[i]);
                i += 1;
            }
        }
        word(&mut out, &key, value)?;
    }
    if out.connection.host.is_empty() && out.connection.database.is_empty() {
        return Err("Bağlantı dizesinde sunucu (host) ya da veritabanı (dbname) yok.".to_owned());
    }
    Ok(out)
}

/// One of libpq's keywords onto the fields; the others are passed over.
fn word(out: &mut Parsed, key: &str, value: String) -> Result<(), String> {
    let c = &mut out.connection;
    match key.to_ascii_lowercase().as_str() {
        "host" | "hostaddr" => c.host = value,
        "port" => {
            c.port = value
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|&p| p > 0)
                .ok_or_else(|| format!("“{value}” bir bağlantı noktası değil."))?;
        }
        "dbname" | "database" => c.database = value,
        "user" | "username" => c.user = value,
        "password" => out.password = Some(value).filter(|p| !p.is_empty()),
        "sslmode" => {
            c.ssl = SslMode::from_libpq(&value).ok_or_else(|| format!("“{value}” bir SSL kipi değil."))?;
        }
        _ => {}
    }
    Ok(())
}

/// Percent-encodes all but RFC 3986's unreserved characters.
fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(text: &str) -> Result<String, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("“{text}” içinde bozuk % kodu var."))?;
            out.push(hex);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| format!("“{text}” UTF-8 değil."))
}

/// What a password is looked up by: the server (its case does not count), port, database and user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
}

impl Key {
    pub fn of(c: &DbConnection) -> Key {
        Key {
            host: c.host.trim().to_ascii_lowercase(),
            port: c.port,
            database: c.database.trim().to_owned(),
            user: c.user.clone(),
        }
    }
}

// ── The device's connections ──────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format: String,
    version: u32,
    connections: Vec<DbConnection>,
}

/// The device's connections, read once and written whole on each change.
pub struct ConnectionStore {
    folder: Option<PathBuf>,
    list: Mutex<Vec<DbConnection>>,
    /// Bumped on each change, so lists follow.
    version: AtomicU64,
    /// The file set aside when it did not read, for the user to be told once.
    set_aside: Mutex<Option<PathBuf>>,
}

impl ConnectionStore {
    /// The connections kept in `folder` (none: in memory only).
    pub fn open(folder: Option<PathBuf>) -> ConnectionStore {
        let (list, set_aside) = match folder.as_deref() {
            Some(f) => read(f),
            None => (Vec::new(), None),
        };
        ConnectionStore {
            folder,
            list: Mutex::new(list),
            version: AtomicU64::new(0),
            set_aside: Mutex::new(set_aside),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<DbConnection>> {
        self.list.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The connections, by name in Turkish order.
    pub fn list(&self) -> Vec<DbConnection> {
        let mut list = self.lock().clone();
        list.sort_by_key(|c| fold(&c.name));
        list
    }

    pub fn get(&self, name: &str) -> Option<DbConnection> {
        self.lock().iter().find(|c| c.name == name).cloned()
    }

    /// Changes so far.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    /// The file that did not read and was set aside, once.
    pub fn take_set_aside(&self) -> Option<PathBuf> {
        self.set_aside.lock().unwrap_or_else(PoisonError::into_inner).take()
    }

    /// Keeps `connection`: a new one, or the one named `previous` (renamed too). Refused: its problems, or a name
    /// another connection has.
    pub fn put(&self, previous: Option<&str>, connection: DbConnection) -> Result<(), String> {
        if let Some((_, why)) = connection.problems().into_iter().next() {
            return Err(why);
        }
        let mut list = self.lock();
        let name = connection.name.trim().to_owned();
        let clash = list
            .iter()
            .any(|c| fold(&c.name) == fold(&name) && Some(c.name.as_str()) != previous);
        if clash {
            return Err(format!("“{name}” adında bir bağlantı zaten var; başka bir ad verin."));
        }
        let mut connection = connection;
        connection.name = name;
        match previous.and_then(|p| list.iter().position(|c| c.name == p)) {
            Some(at) => list[at] = connection,
            None => list.push(connection),
        }
        self.version.fetch_add(1, Ordering::Relaxed);
        self.write(&list)
    }

    /// Forgets the connection named `name`; whether there was one.
    pub fn remove(&self, name: &str) -> Result<bool, String> {
        let mut list = self.lock();
        let before = list.len();
        list.retain(|c| c.name != name);
        if list.len() == before {
            return Ok(false);
        }
        self.version.fetch_add(1, Ordering::Relaxed);
        self.write(&list).map(|()| true)
    }

    /// `base`, or `base (2)`, `base (3)` …: the first name no connection has.
    pub fn unique_name(&self, base: &str) -> String {
        let list = self.lock();
        let base = base.trim();
        let taken = |n: &str| list.iter().any(|c| fold(&c.name) == fold(n));
        if !taken(base) {
            return base.to_owned();
        }
        (2..)
            .map(|i| format!("{base} ({i})"))
            .find(|n| !taken(n))
            .unwrap_or_default()
    }

    /// The saved password of a connection reaching `key`: one naming the user before one for any user.
    pub fn password_for(&self, key: &Key) -> Option<String> {
        let list = self.lock();
        let mut fits: Vec<&DbConnection> = list
            .iter()
            .filter(|c| c.password.is_some() && c.reaches(key))
            .collect();
        fits.sort_by_key(|c| c.user.is_empty());
        fits.first().and_then(|c| c.password.clone())
    }

    fn write(&self, list: &[DbConnection]) -> Result<(), String> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let file = StoreFile {
            format: FORMAT.into(),
            version: 1,
            connections: list.to_vec(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        write_private(&folder.join(FILE), text.as_bytes()).map_err(|e| {
            format!(
                "Veritabanı bağlantıları kaydedilemedi ({}): {e}. Klasörün yazılabilir olduğunu denetleyin.",
                folder.display()
            )
        })
    }
}

/// A name for comparing and ordering: Turkish lower case, so “İl” and “il” are one.
fn fold(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| match c {
            'I' => 'ı',
            'İ' => 'i',
            other => other.to_lowercase().next().unwrap_or(other),
        })
        .collect()
}

/// Writes `bytes` to `path` readable by the user alone, through a file beside it.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("json.yeni");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut f = options.open(&temp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    #[cfg(unix)]
    std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&temp, path)
}

/// The connections of the file in `folder`; one that does not read is set aside and said.
fn read(folder: &Path) -> (Vec<DbConnection>, Option<PathBuf>) {
    let path = folder.join(FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        return (Vec::new(), None);
    };
    match serde_json::from_slice::<StoreFile>(&bytes) {
        Ok(f) if f.format == FORMAT && f.version == 1 => (f.connections, None),
        _ => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let aside = folder.join(format!("veritabani-baglantilari-okunamadi-{stamp}.json"));
            let moved = std::fs::rename(&path, &aside).is_ok();
            (Vec::new(), moved.then_some(aside))
        }
    }
}

/// The device's configuration folder: `$XDG_CONFIG_HOME/kentos-cad`, else `~/.config/kentos-cad` (the desktop's
/// settings' folder).
pub fn device_folder() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("kentos-cad"))
}

static MEMORY: AtomicBool = AtomicBool::new(false);
static STORE: OnceLock<ConnectionStore> = OnceLock::new();

/// The process's connections are kept in memory, never in the device's file: called by tests before anything
/// reads them.
pub fn keep_in_memory() {
    MEMORY.store(true, Ordering::Relaxed);
}

/// The process's connections: the device's, opened on first use.
pub fn store() -> &'static ConnectionStore {
    STORE.get_or_init(|| {
        ConnectionStore::open(if MEMORY.load(Ordering::Relaxed) {
            None
        } else {
            device_folder()
        })
    })
}

// ── Passwords for this session ────────────────────────────────────────

fn session() -> &'static Mutex<Vec<(Key, String)>> {
    static SESSION: OnceLock<Mutex<Vec<(Key, String)>>> = OnceLock::new();
    SESSION.get_or_init(|| Mutex::new(Vec::new()))
}

/// Keeps `password` for `connection`'s database and user until the program ends (a password typed but not saved).
pub fn remember(connection: &DbConnection, password: String) {
    let key = Key::of(connection);
    let mut list = session().lock().unwrap_or_else(PoisonError::into_inner);
    list.retain(|(k, _)| *k != key);
    list.push((key, password));
}

/// Forgets the session's password for `connection` (it was refused).
pub fn forget(connection: &DbConnection) {
    let key = Key::of(connection);
    session()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|(k, _)| *k != key);
}

/// The password KentOS knows for `key`: this session's, else the device's saved one; none leaves `PGPASSWORD` and
/// `~/.pgpass` to the caller.
pub fn password_for(key: &Key) -> Option<String> {
    let typed = session()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(k, _)| k.host == key.host && k.port == key.port && k.database == key.database && (k.user.is_empty() || k.user == key.user))
        .map(|(_, p)| p.clone());
    typed.or_else(|| store().password_for(key))
}

/// Whether KentOS can give a password for `connection` without asking (session, device, `PGPASSWORD`).
pub fn knows_password(connection: &DbConnection) -> bool {
    connection.password.is_some()
        || password_for(&Key::of(connection)).is_some()
        || std::env::var_os("PGPASSWORD").is_some_and(|p| !p.is_empty())
}

// ── Dışa aktar and İçe aktar ─────────────────────────────────────────

/// `connections` as a file to give others (the device file's format); their saved passwords only when `passwords`.
pub fn export(connections: &[DbConnection], passwords: bool) -> String {
    let list: Vec<DbConnection> = connections
        .iter()
        .map(|c| DbConnection {
            password: if passwords { c.password.clone() } else { None },
            ..c.clone()
        })
        .collect();
    let file = StoreFile {
        format: FORMAT.into(),
        version: 1,
        connections: list,
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

/// A connection read from a file given, and why it was not taken when it was not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub connection: DbConnection,
    pub skipped: Option<String>,
}

/// The connections of a file [`export`] wrote; why not, when it is no such file.
pub fn import(text: &str) -> Result<Vec<Imported>, String> {
    let file: StoreFile = serde_json::from_str(text).map_err(|e| {
        format!("Dosya bir KentOS bağlantı dosyası olarak okunamadı: {e}. Bağlantıları dışa aktar ile yazılan dosyayı seçin.")
    })?;
    if file.format != FORMAT {
        return Err(format!(
            "Bu dosya KentOS'un veritabanı bağlantıları değil (biçim “{}”, beklenen “{FORMAT}”).",
            file.format
        ));
    }
    if file.version != 1 {
        return Err(format!(
            "Dosyanın sürümü {}; bu KentOS 1. sürümü okur. KentOS'u güncelleyin.",
            file.version
        ));
    }
    Ok(file
        .connections
        .into_iter()
        .map(|c| {
            let skipped = c
                .problems()
                .into_iter()
                .next()
                .map(|(_, why)| format!("“{}”: {why}", c.name));
            Imported {
                connection: c,
                skipped,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kent() -> DbConnection {
        DbConnection {
            name: "Belediye".into(),
            host: "db.belediye.local".into(),
            database: "kent".into(),
            user: "ali".into(),
            ..DbConnection::default()
        }
    }

    #[test]
    fn the_address_is_built_from_the_fields_and_never_holds_the_password() {
        let mut c = kent();
        c.password = Some("gizli".into());
        assert_eq!(c.address(), "postgresql://ali@db.belediye.local:5432/kent");
        assert!(kentos_native_application::provider::address_problem(&c.address()).is_none());
        c.ssl = SslMode::VerifyFull;
        c.port = 6543;
        c.database = "Kent Veri".into();
        c.user = "ali@kurum".into();
        assert_eq!(
            c.address(),
            "postgresql://ali%40kurum@db.belediye.local:6543/Kent%20Veri?sslmode=verify-full"
        );
        assert!(kentos_native_application::provider::address_problem(&c.address()).is_none());
        // Turkish letters go through as UTF-8.
        c.database = "çevre".into();
        assert!(c.address().contains("/%C3%A7evre?"));
        // An IPv6 server in brackets; no user: libpq's default.
        c.user.clear();
        c.host = "fd00::5".into();
        assert!(c.address().starts_with("postgresql://[fd00::5]:6543/"));
    }

    #[test]
    fn an_address_or_a_connection_string_reads_back_into_the_fields() {
        let c = DbConnection {
            ssl: SslMode::Require,
            port: 6543,
            database: "Kent Veri".into(),
            user: "ali@kurum".into(),
            ..kent()
        };
        let back = parse(&c.address()).expect("reads");
        assert_eq!(back.connection.address(), c.address());
        assert_eq!(back.password, None);
        // A pasted address with a password gives it apart.
        let p = parse("postgres://cbs:s%C3%BCper@10.0.0.5/kent?sslmode=disable").expect("reads");
        assert_eq!(p.connection.host, "10.0.0.5");
        assert_eq!(p.connection.port, DEFAULT_PORT);
        assert_eq!(p.connection.user, "cbs");
        assert_eq!(p.connection.ssl, SslMode::Disable);
        assert_eq!(p.password.as_deref(), Some("süper"));
        // A connection string of libpq's words, a table's after it.
        let q = parse(
            r#"dbname='kent' host=db.belediye.local port=5433 user='ali' password='a b\'c' sslmode=verify-ca key='fid' srid=5256 type=MultiPolygon checkPrimaryKeyUnicity='1' table="kadastro"."parsel" (geom)"#,
        )
        .expect("reads");
        assert_eq!(q.connection.database, "kent");
        assert_eq!(q.connection.port, 5433);
        assert_eq!(q.connection.ssl, SslMode::VerifyCa);
        assert_eq!(q.password.as_deref(), Some("a b'c"));
        assert!(parse("dosya.gpkg").is_err());
        assert!(parse("postgresql://db:yok/kent").is_err());
        assert!(parse("host='açık").is_err());
    }

    #[test]
    fn a_connection_says_what_it_lacks_field_by_field() {
        assert!(kent().problems().is_empty());
        let empty = DbConnection::default();
        let fields: Vec<Field> = empty.problems().into_iter().map(|(f, _)| f).collect();
        assert_eq!(fields, [Field::Name, Field::Host, Field::Database]);
        let bad = DbConnection {
            host: "db belediye".into(),
            port: 0,
            ..kent()
        };
        let fields: Vec<Field> = bad.problems().into_iter().map(|(f, _)| f).collect();
        assert_eq!(fields, [Field::Host, Field::Port]);
    }

    #[test]
    fn the_store_keeps_names_unique_writes_its_file_for_the_user_alone_and_finds_passwords() {
        let dir = std::env::temp_dir().join(format!("kentos-db-baglanti-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = ConnectionStore::open(Some(dir.clone()));
        let mut a = kent();
        a.password = Some("gizli".into());
        s.put(None, a.clone()).expect("kept");
        // A second connection of the same name (Turkish case) is refused.
        let clash = DbConnection {
            name: "BELEDİYE".into(),
            ..kent()
        };
        assert!(s.put(None, clash).unwrap_err().contains("zaten var"));
        assert_eq!(s.unique_name("belediye"), "belediye (2)");
        // Renamed in place.
        let renamed = DbConnection {
            name: "İl özel idaresi".into(),
            ..a.clone()
        };
        s.put(Some("Belediye"), renamed).expect("renamed");
        assert_eq!(s.list().len(), 1);
        assert!(s.get("Belediye").is_none());
        // A password by server (case aside), port, database and user.
        let key = Key {
            host: "DB.belediye.local".to_ascii_lowercase(),
            port: 5432,
            database: "kent".into(),
            user: "ali".into(),
        };
        assert_eq!(s.password_for(&key).as_deref(), Some("gizli"));
        assert!(s.password_for(&Key { user: "veli".into(), ..key.clone() }).is_none());
        assert!(s.password_for(&Key { port: 5433, ..key.clone() }).is_none());
        // Read again from the file, which only the user may read.
        let again = ConnectionStore::open(Some(dir.clone()));
        assert_eq!(again.list(), s.list());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(FILE)).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert_eq!(again.remove("İl özel idaresi"), Ok(true));
        assert!(ConnectionStore::open(Some(dir.clone())).list().is_empty());
        // A file that does not read is set aside, not written over.
        std::fs::write(dir.join(FILE), b"{bozuk").unwrap();
        let fresh = ConnectionStore::open(Some(dir.clone()));
        assert!(fresh.list().is_empty());
        let aside = fresh.take_set_aside().expect("set aside");
        assert!(aside.exists());
        assert!(fresh.take_set_aside().is_none(), "said once");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_team_is_given_the_same_connections_with_or_without_their_passwords() {
        let mut a = kent();
        a.password = Some("gizli".into());
        a.ssl = SslMode::VerifyFull;
        a.geometryless = true;
        let b = DbConnection {
            name: "Yerel".into(),
            host: "localhost".into(),
            database: "gis".into(),
            ..DbConnection::default()
        };
        let without = export(&[a.clone(), b.clone()], false);
        assert!(!without.contains("gizli"), "{without}");
        let back = import(&without).expect("reads");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].connection, DbConnection { password: None, ..a.clone() });
        assert_eq!(back[1].connection, b);
        assert!(back.iter().all(|i| i.skipped.is_none()));
        let with = import(&export(std::slice::from_ref(&a), true)).expect("reads");
        assert_eq!(with[0].connection, a);
        // A connection the file cannot give whole is said, not taken.
        let broken = export(&[DbConnection { host: String::new(), ..kent() }], false);
        assert!(import(&broken).expect("reads")[0].skipped.as_deref().is_some_and(|w| w.contains("Sunucu")));
        assert!(import("{bozuk").unwrap_err().contains("okunamadı"));
        assert!(import(r#"{"format":"başka","version":1,"connections":[]}"#).unwrap_err().contains("değil"));
        assert!(import(r#"{"format":"kentos.db-connections","version":2,"connections":[]}"#).unwrap_err().contains("güncelleyin"));
    }

    #[test]
    fn the_modes_read_as_libpq_says_them() {
        for m in SSL_MODES {
            assert_eq!(SslMode::from_libpq(m.libpq()), Some(m));
        }
        assert_eq!(SslMode::from_libpq(" VERIFY-FULL "), Some(SslMode::VerifyFull));
        assert_eq!(SslMode::from_libpq("SslPrefer"), None);
    }

    #[test]
    fn a_typed_password_is_kept_for_the_session_and_forgotten_when_refused() {
        keep_in_memory();
        let c = DbConnection {
            host: "oturum.example".into(),
            port: 7000 + (std::process::id() % 1000) as u16,
            ..kent()
        };
        let key = Key::of(&c);
        assert!(password_for(&key).is_none());
        remember(&c, "bu-oturum".into());
        assert_eq!(password_for(&key).as_deref(), Some("bu-oturum"));
        forget(&c);
        assert!(password_for(&key).is_none());
    }
}
