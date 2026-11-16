//! The box's Landlock domain (docs/adr/8057 §61): the files the rule needs, no TCP; signals and abstract UNIX sockets
//! (ABI 6) and pathname UNIX sockets (ABI 9) kept inside where the kernel can.

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset,
    RulesetAttr, RulesetCreated, RulesetCreatedAttr, RulesetError, Scope,
};

use super::{Allowed, Landlock};

/// Every box's ruleset: the file rights and TCP of ABI 4 required; newer file rights (device ioctl, pathname UNIX
/// sockets) and the scopes (abstract UNIX sockets, signals) where the kernel has them.
fn ruleset() -> Result<RulesetCreated, RulesetError> {
    Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V4))?
        .handle_access(AccessNet::from_all(ABI::V4))?
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(ABI::V9))?
        .scope(Scope::from_all(ABI::V9))?
        .create()
}

/// Whether the kernel has the scopes (ABI 6) and pathname UNIX sockets (ABI 9), asked without restricting anything.
fn has_scopes() -> bool {
    Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|r| r.scope(Scope::from_all(ABI::V6)))
        .and_then(Ruleset::create)
        .is_ok()
}

fn has_unix_paths() -> bool {
    Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::ResolveUnix)
        .and_then(Ruleset::create)
        .is_ok()
}

/// What Landlock closes here, or why it cannot close: the files and TCP of ABI 4 at least.
pub fn landlock() -> Result<Landlock, String> {
    ruleset().map_err(|e| {
        format!("Landlock (ABI 4: dosyalar ve TCP) bu çekirdekte yok ya da kapalı: {e}")
    })?;
    Ok(Landlock {
        scopes: has_scopes(),
        unix_paths: has_unix_paths(),
    })
}

/// Restricts the calling thread (the box mode's only one) to `allowed`, irreversibly; inherited across exec.
pub fn restrict(allowed: &Allowed) -> Result<(), String> {
    let read_dir = AccessFs::from_read(ABI::V9);
    let read_file = read_dir & AccessFs::from_file(ABI::V9);
    let mut ruleset = ruleset().map_err(|e| e.to_string())?;
    for path in &allowed.read {
        let access = if path.is_dir() { read_dir } else { read_file };
        let fd = PathFd::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|e| e.to_string())?;
    }
    for path in &allowed.write {
        let access = if path.is_dir() {
            AccessFs::from_all(ABI::V9)
        } else {
            AccessFs::from_file(ABI::V9)
        };
        let fd = PathFd::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|e| e.to_string())?;
    }
    let status = ruleset.restrict_self().map_err(|e| e.to_string())?;
    if status.ruleset == landlock::RulesetStatus::NotEnforced {
        return Err("Landlock kuralları uygulanmadı".to_owned());
    }
    Ok(())
}
