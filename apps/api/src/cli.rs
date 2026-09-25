//! Administration commands of `kentosd` (run as the owner role). Passwords
//! are read from standard input or a named environment variable, never from
//! the command line, where other users of the machine could see them.

use std::io::BufRead;
use std::time::Duration;

use kentos_application::admin;
use kentos_contracts::TenantRole;
use kentos_postgres::setup::{self, DEFAULT_DATABASE};
use kentos_postgres::{Db, env, migrate};

use crate::config::Config;

/// Words and `--flag value` / `--switch` options.
pub struct Args {
    pub words: Vec<String>,
    options: Vec<(String, Option<String>)>,
}

const SWITCHES: [&str; 2] = ["--seat", "--password-stdin"];

impl Args {
    pub fn parse(raw: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut words = Vec::new();
        let mut options = Vec::new();
        let mut raw = raw.peekable();
        while let Some(a) = raw.next() {
            // A bare `--` separates arguments, it is none itself: `pnpm kentosd -- migrate` passes it through.
            if a == "--" {
                continue;
            }
            if let Some(name) = a.strip_prefix("--") {
                if SWITCHES.contains(&a.as_str()) {
                    options.push((name.to_string(), None));
                } else {
                    let value = raw
                        .next()
                        .ok_or_else(|| format!("{a} bir değer bekliyor"))?;
                    options.push((name.to_string(), Some(value)));
                }
            } else {
                words.push(a);
            }
        }
        Ok(Self { words, options })
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.as_deref())
    }

    fn required(&self, name: &str) -> Result<&str, String> {
        self.value(name).ok_or_else(|| format!("--{name} gerekli"))
    }

    fn switch(&self, name: &str) -> bool {
        self.options.iter().any(|(n, v)| n == name && v.is_none())
    }
}

fn password(args: &Args) -> Result<String, String> {
    if let Some(var) = args.value("password-env") {
        return std::env::var(var).map_err(|_| format!("{var} ortam değişkeni tanımlı değil"));
    }
    if args.switch("password-stdin") {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        return Ok(line.trim_end_matches(['\r', '\n']).to_string());
    }
    Err("Parola --password-stdin ya da --password-env DEĞİŞKEN ile verilir (komut satırında görünmesin diye).".into())
}

fn on_off(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "on" | "acik" | "açık" => Ok(true),
        "off" | "kapali" | "kapalı" => Ok(false),
        _ => Err(format!("--{name} on ya da off olmalı: {value}")),
    }
}

fn role(text: &str) -> Result<TenantRole, String> {
    kentos_application::tenancy::role_from_db(text).ok_or_else(|| {
        format!("Rol geçersiz: {text} (owner, admin, project_manager, editor, viewer)")
    })
}

async fn owner_pool(config: &Config) -> Result<sqlx::PgPool, String> {
    let url = config
        .owner_url
        .as_deref()
        .ok_or("KENTOS_DATABASE_OWNER_URL yok: önce `kentosd db-setup` çalıştırın.")?;
    Ok(Db::connect(url, 2, Duration::from_secs(10))
        .await
        .map_err(|e| format!("Veritabanına bağlanılamadı: {e}"))?
        .pool)
}

pub async fn run(config: &Config, args: &Args) -> Result<(), String> {
    let words: Vec<&str> = args.words.iter().map(String::as_str).collect();
    let fail = |e: kentos_application::AppError| e.to_string();
    match words.as_slice() {
        ["db-setup"] => {
            let admin_url = args
                .value("admin-url")
                .map(str::to_string)
                .or_else(|| std::env::var("KENTOS_ADMIN_URL").ok())
                .unwrap_or_else(|| "postgres://postgres:postgres@127.0.0.1:5432/postgres".into());
            let database = args.value("database").unwrap_or(DEFAULT_DATABASE);
            let report = setup::setup(&admin_url, database, &config.vars).await?;
            let mut vars = config.vars.clone();
            vars.extend(report.vars);
            env::write_file(
                &config.env_file,
                &vars,
                "KentOS yerel ayarları (kentosd db-setup yazar). Depoya girmez; parolalar içerir.",
            )
            .map_err(|e| format!("{} yazılamadı: {e}", config.env_file.display()))?;
            for r in &report.created_roles {
                println!("rol açıldı: {r}");
            }
            println!(
                "veritabanı {}: {database}",
                if report.created_database {
                    "açıldı"
                } else {
                    "hazır"
                }
            );
            println!("bağlantı bilgileri: {}", config.env_file.display());
        }
        ["migrate"] => {
            let pool = owner_pool(config).await?;
            migrate(&pool)
                .await
                .map_err(|e| format!("Migration uygulanamadı: {e}"))?;
            println!(
                "şema güncel ({} migration)",
                kentos_postgres::MIGRATOR.iter().count()
            );
        }
        ["tenant", "add"] => {
            let seats: i32 = args
                .required("seats")?
                .parse()
                .map_err(|_| "--seats bir sayı olmalı")?;
            let id = admin::create_tenant(
                &owner_pool(config).await?,
                args.required("slug")?,
                args.required("name")?,
                seats,
            )
            .await
            .map_err(fail)?;
            println!("kurum açıldı: {id}");
        }
        ["tenant", "list"] => {
            for t in admin::list_tenants(&owner_pool(config).await?)
                .await
                .map_err(fail)?
            {
                let kind = if t.kind == "personal" {
                    "kişisel"
                } else {
                    "kurum"
                };
                let admins = if t.kind == "personal" {
                    "-"
                } else if t.admins_access_all_projects {
                    "yöneticiler tüm projelerde"
                } else {
                    "yöneticiler yalnız paylaşılanlarda"
                };
                let download = if t.viewer_download {
                    "görüntüleyici indirir"
                } else {
                    "görüntüleyici indiremez"
                };
                println!(
                    "{}\t{}\t{kind}\t{}/{} koltuk\t{} üye\t{admins}\t{download}",
                    t.slug, t.name, t.seats_used, t.seat_limit, t.members
                );
            }
        }
        ["tenant", "policy"] => {
            admin::set_tenant_policy(
                &owner_pool(config).await?,
                args.required("slug")?,
                args.value("admins-access-all")
                    .map(|v| on_off("admins-access-all", v))
                    .transpose()?,
                args.value("viewer-download")
                    .map(|v| on_off("viewer-download", v))
                    .transpose()?,
            )
            .await
            .map_err(fail)?;
            println!("kurum politikası kaydedildi");
        }
        ["user", "add"] => {
            let pool = owner_pool(config).await?;
            let id = admin::create_local_user(
                &pool,
                args.required("login")?,
                args.required("name")?,
                args.value("email"),
                &password(args)?,
            )
            .await
            .map_err(fail)?;
            println!("hesap açıldı: {id}");
        }
        ["user", "password"] => {
            admin::set_password(
                &owner_pool(config).await?,
                args.required("login")?,
                &password(args)?,
            )
            .await
            .map_err(fail)?;
            println!("parola değişti; hesabın açık oturumları kapatıldı");
        }
        ["member", "add"] => {
            let pool = owner_pool(config).await?;
            admin::set_membership(
                &pool,
                args.required("tenant")?,
                args.required("user")?,
                role(args.required("role")?)?,
                args.switch("seat"),
            )
            .await
            .map_err(fail)?;
            println!("üyelik kaydedildi");
        }
        ["member", "list"] => {
            for m in admin::list_members(&owner_pool(config).await?, args.required("tenant")?)
                .await
                .map_err(fail)?
            {
                println!(
                    "{}\t{}\t{:?}\t{}\t{}",
                    m.login.as_deref().unwrap_or("(OpenID)"),
                    m.name,
                    m.role,
                    if m.seat { "koltuk" } else { "koltuksuz" },
                    m.status
                );
            }
        }
        ["project", "deleted"] => {
            for p in admin::deleted_projects(&owner_pool(config).await?, args.required("tenant")?)
                .await
                .map_err(fail)?
            {
                let at = p
                    .deleted_at
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default();
                println!("{}\t{}\t{at}\t{}", p.id, p.name, p.deleted_by);
            }
        }
        ["project", "restore"] => {
            let project = uuid::Uuid::parse_str(args.required("project")?)
                .map_err(|_| "--project bir proje kimliği (UUID) olmalı".to_string())?;
            let name = admin::restore_project(
                &owner_pool(config).await?,
                args.required("tenant")?,
                project,
            )
            .await
            .map_err(fail)?;
            println!("“{name}” geri getirildi; listelerde görünür ve açılır");
        }
        ["dev-seed"] => dev_seed(config).await?,
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

/// Development data: tenant `ornek-buro` with three accounts: a project
/// manager and an editor (two-editor tests) and an admin (who may delete
/// projects). Their shared password is random and kept in `.env.local`
/// (`KENTOS_DEV_PASSWORD`). Safe to run again.
async fn dev_seed(config: &Config) -> Result<(), String> {
    let pool = owner_pool(config).await?;
    let tenants = admin::list_tenants(&pool)
        .await
        .map_err(|e| e.to_string())?;
    if !tenants.iter().any(|t| t.slug == "ornek-buro") {
        admin::create_tenant(&pool, "ornek-buro", "Örnek Harita Bürosu", 5)
            .await
            .map_err(|e| e.to_string())?;
    }
    let mut vars = config.vars.clone();
    let password = vars
        .get("KENTOS_DEV_PASSWORD")
        .cloned()
        .unwrap_or_else(|| format!("dev-{}", &setup::random_secret()[..20]));
    for (login, name, role) in [
        ("ayse", "Ayşe Yılmaz", TenantRole::ProjectManager),
        ("mehmet", "Mehmet Demir", TenantRole::Editor),
        ("zeynep", "Zeynep Kaya", TenantRole::Admin),
    ] {
        match admin::create_local_user(&pool, login, name, None, &password).await {
            Ok(_) => {}
            Err(kentos_application::AppError::Invalid(m)) if m.contains("zaten") => {
                admin::set_password(&pool, login, &password)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }
        admin::set_membership(&pool, "ornek-buro", login, role, true)
            .await
            .map_err(|e| e.to_string())?;
    }
    vars.insert("KENTOS_DEV_PASSWORD".into(), password);
    env::write_file(
        &config.env_file,
        &vars,
        "KentOS yerel ayarları (kentosd db-setup yazar). Depoya girmez; parolalar içerir.",
    )
    .map_err(|e| format!("{} yazılamadı: {e}", config.env_file.display()))?;
    println!(
        "kurum: ornek-buro; hesaplar: ayse (proje yöneticisi), mehmet (editör), zeynep (yönetici)"
    );
    println!(
        "parola: {} içinde KENTOS_DEV_PASSWORD",
        config.env_file.display()
    );
    Ok(())
}

pub const USAGE: &str = "kullanım:
  kentosd [serve]                             API'yi başlatır
  kentosd db-setup [--admin-url URL] [--database AD]
  kentosd migrate
  kentosd tenant add --slug KISA --name AD --seats N
  kentosd tenant list                         kurumlar ve kişisel alanlar, koltuklar, politikalar
  kentosd tenant policy --slug KISA [--admins-access-all on|off] [--viewer-download on|off]
  kentosd user add --login GİRİŞ --name AD [--email E] (--password-stdin | --password-env DEĞİŞKEN)
  kentosd user password --login GİRİŞ (--password-stdin | --password-env DEĞİŞKEN)
  kentosd member add --tenant KISA --user GİRİŞ|KİMLİK --role owner|admin|project_manager|editor|viewer [--seat]
  kentosd member list --tenant KISA
  kentosd project deleted --tenant KISA       silinmiş projeler: kimlik, ad, silinme zamanı, silen
  kentosd project restore --tenant KISA --project KİMLİK
  kentosd dev-seed                            geliştirme kurumu ve üç hesap (ayse, mehmet, zeynep)";

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Args {
        Args::parse(list.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn a_bare_double_dash_is_not_a_command_or_an_option() {
        let a = args(&["--", "migrate"]);
        assert_eq!(a.words, ["migrate"]);
        let a = args(&["project", "--", "restore", "--tenant", "buro", "--seat"]);
        assert_eq!(a.words, ["project", "restore"]);
        assert_eq!((a.value("tenant"), a.switch("seat")), (Some("buro"), true));
        assert!(Args::parse(["--tenant".to_string()].into_iter()).is_err());
    }
}
