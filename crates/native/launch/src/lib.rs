//! What KentOS Launcher (`apps/launcher`, docs/adr/0800) and the programs it
//! starts agree on, and nothing more: the KentOS modules a project belongs to
//! ([`Module`]), the command line that starts a program in one ([`args`]) and
//! the recent files a program keeps for the launcher to show ([`recent`]), and
//! where a program is found ([`programs`]).
//! Programs that are not KentOS's own (any language) need none of it: the
//! launcher starts them from their manifests.

pub mod args;
mod module;
pub mod programs;
pub mod recent;

pub use module::Module;
