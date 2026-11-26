//! What KentOS Launcher (`apps/launcher`, docs/adr/0800) and the programs it
//! starts agree on, and nothing more: the KentOS modules a project belongs to
//! ([`Module`]), the command line that starts a program in one ([`args`]) and
//! the recent files a program keeps for the launcher to show ([`recent`]),
//! where a program is found ([`programs`]) and where an installation keeps what it reads ([`install`]).
//! Programs that are not KentOS's own (any language) need none of it: the
//! launcher starts them from their manifests.

pub mod args;
pub mod install;
mod module;
pub mod programs;
pub mod recent;

pub use module::{Client, Kept, LayerKind, Mobile, Module, ModuleSet, Storage, Writes};
