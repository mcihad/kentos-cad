//! The box's Landlock domain (docs/adr/8057 §61, §65): the files the rule needs. ABI 1's file rights are required;
//! what newer ABIs add is used where the kernel has it: renames and links across folders (2; below it they are
//! denied outright), truncation (3), TCP (4), device ioctl (5), signals and abstract UNIX sockets kept inside (6),
//! pathname UNIX sockets (9). The seccomp filter closes what an older kernel leaves open (`seccomp`).

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset,
    RulesetAttr, RulesetCreated, RulesetCreatedAttr, RulesetError, Scope,
};

use super::{Allowed, Landlock};

/// The box's ruleset, at most at `cap`: ABI 1's file rights required; the newer file rights, TCP and the scopes
/// where the kernel has them.
fn ruleset(cap: ABI) -> Result<RulesetCreated, RulesetError> {
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V1))?
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(cap))?;
    if cap >= ABI::V4 {
        ruleset = ruleset.handle_access(AccessNet::from_all(cap))?;
    }
    if cap >= ABI::V6 {
        ruleset = ruleset.scope(Scope::from_all(cap))?;
    }
    ruleset.create()
}

/// What Landlock closes here, or why it cannot close (no ABI 1). The newest ABI whose additions the box uses is asked
/// feature by feature, without restricting anything: 9, 6, 5, 4, 3, 2 or 1 (7 and 8 add nothing the box uses).
pub fn landlock() -> Result<Landlock, String> {
    let hard = || Ruleset::default().set_compatibility(CompatLevel::HardRequirement);
    let has = |ruleset: Result<Ruleset, RulesetError>| ruleset.and_then(Ruleset::create).is_ok();
    hard()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(Ruleset::create)
        .map_err(|e| format!("Landlock bu çekirdekte yok ya da kapalı: {e}"))?;
    let abi = if has(hard().handle_access(AccessFs::ResolveUnix)) {
        9
    } else if has(hard()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|r| r.scope(Scope::from_all(ABI::V6))))
    {
        6
    } else if has(hard().handle_access(AccessFs::IoctlDev)) {
        5
    } else if has(hard().handle_access(AccessNet::from_all(ABI::V4))) {
        4
    } else if has(hard().handle_access(AccessFs::Truncate)) {
        3
    } else if has(hard().handle_access(AccessFs::Refer)) {
        2
    } else {
        1
    };
    Ok(Landlock { abi })
}

/// Restricts the calling thread (the box mode's only one) to `allowed`, at most at Landlock ABI `cap`, irreversibly;
/// inherited across exec.
pub fn restrict(allowed: &Allowed, cap: u8) -> Result<(), String> {
    let cap = ABI::from(i32::from(cap));
    let read_dir = AccessFs::from_read(cap);
    let read_file = read_dir & AccessFs::from_file(cap);
    let mut ruleset = ruleset(cap).map_err(|e| e.to_string())?;
    for path in &allowed.read {
        let access = if path.is_dir() { read_dir } else { read_file };
        let fd = PathFd::new(path).map_err(|e| format!("{}: {e}", path.display()))?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|e| e.to_string())?;
    }
    for path in &allowed.write {
        let access = if path.is_dir() {
            AccessFs::from_all(cap)
        } else {
            AccessFs::from_file(cap)
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
