//! A throwaway database for the cloud end-to-end test (`KENTOS_E2E_DB=scratch
//! pnpm e2e:cloud`, apps/web/scripts/e2e/cloud.mjs): a new
//! `kentos_cad_test_*` database with every migration of this build applied
//! (as the server tests make them, `kentos_postgres::testing`) and the
//! development accounts of `kentosd dev-seed` (organisation `ornek-buro`;
//! ayse, mehmet, zeynep) with the password in `KENTOS_DEV_PASSWORD`. The
//! development database `kentos_cad` is never touched, so a migration can be
//! tried end to end before it is applied there.
//!
//! Prints the database's name (never a password or an address) on the first
//! line, then waits until its standard input closes and drops the database.
//! A run that dies leaves it behind; the next test database made after an
//! hour removes it.
//!
//!   cargo run -p kentos-api --example e2e_database

use std::io::Read;

use kentos_application::admin;
use kentos_contracts::TenantRole;
use kentos_postgres::testing::TestDb;

#[tokio::main]
async fn main() {
    let password = std::env::var("KENTOS_DEV_PASSWORD")
        .expect("KENTOS_DEV_PASSWORD gerekli (geliştirme hesaplarının parolası)");
    // No local server: a failure here, not a skipped test.
    let db = TestDb::create()
        .await
        .expect("test veritabanı kurulamadı: yerel PostgreSQL'e ulaşılamadı (yukarıdaki uyarı nedenini söyler)");
    admin::create_tenant(&db.owner, "ornek-buro", "Örnek Harita Bürosu", 5)
        .await
        .expect("kurum açılamadı");
    for (login, name, role) in [
        ("ayse", "Ayşe Yılmaz", TenantRole::ProjectManager),
        ("mehmet", "Mehmet Demir", TenantRole::Editor),
        ("zeynep", "Zeynep Kaya", TenantRole::Admin),
    ] {
        admin::create_local_user(&db.owner, login, name, None, &password)
            .await
            .expect("hesap açılamadı");
        admin::set_membership(&db.owner, "ornek-buro", login, role, true)
            .await
            .expect("üyelik kaydedilemedi");
    }
    println!("{}", db.name);
    // Until whoever started this closes its end.
    let _ = tokio::task::spawn_blocking(|| {
        let mut rest = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut rest);
    })
    .await;
    db.close().await;
}
