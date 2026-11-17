//! `kentosd extension-catalog …` (docs/adr/8059 §11, §13): the server administrator's grants in the persistent
//! extension catalog, as the owner role. A role is granted in one catalog (`public`, `tenant:KISA`) or in `every` one
//! (the catalog's services: the scanner); an account uploads under a publisher it is a member of; a catalog may trust
//! a publisher (the review's fast path, D12). An organisation's members see its catalog without a grant.

use super::{Args, on_off, owner_pool};
use crate::config::Config;
use crate::extension_catalog::grants;
use crate::extension_catalog::store::Where;

pub async fn run(config: &Config, args: &Args, command: &str) -> Result<(), String> {
    let pool = owner_pool(config).await?;
    match command {
        "grant" => {
            let place = Where::parse(args.required("catalog")?)?;
            let role = grants::role(args.required("role")?)?;
            grants::grant(&pool, &place, args.required("user")?, role).await?;
            println!("rol verildi");
        }
        "revoke" => {
            let place = Where::parse(args.required("catalog")?)?;
            let role = grants::role(args.required("role")?)?;
            let held = grants::revoke(&pool, &place, args.required("user")?, role).await?;
            println!(
                "{}",
                if held {
                    "rol geri alındı"
                } else {
                    "hesabın bu rolü yoktu"
                }
            );
        }
        "publisher" => {
            let publisher = args.required("publisher")?;
            grants::publisher(&pool, publisher, args.value("name")).await?;
            match args.value("user") {
                Some(user) => {
                    let on = on_off("member", args.value("member").unwrap_or("on"))?;
                    let changed = grants::member(&pool, publisher, user, on).await?;
                    println!(
                        "{publisher}: {}",
                        match (on, changed) {
                            (true, true) => "üye eklendi",
                            (true, false) => "zaten üye",
                            (false, true) => "üyelikten çıkarıldı",
                            (false, false) => "üye değildi",
                        }
                    );
                }
                None => println!("{publisher}: yayımcı kaydedildi"),
            }
        }
        "trust" => {
            let place = Where::parse(args.required("catalog")?)?;
            let on = on_off("trusted", args.value("trusted").unwrap_or("on"))?;
            grants::trust(&pool, &place, args.required("publisher")?, on).await?;
            println!(
                "{}",
                if on {
                    "katalog yayımcıya güveniyor"
                } else {
                    "katalog yayımcıya artık güvenmiyor"
                }
            );
        }
        "list" => {
            for line in grants::listing(&pool).await? {
                println!("{}\t{}", line.what, line.text);
            }
        }
        _ => return Err(super::USAGE.into()),
    }
    Ok(())
}
