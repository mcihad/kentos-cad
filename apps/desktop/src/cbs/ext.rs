//! The CBS module's `App` methods, every file's extension trait (docs/adr/9199 §3.5): a file of the module, its parts
//! and its tests take them all with `use crate::cbs_ext::*;`. A file's methods stay its own; this only names them.

pub(crate) use crate::cbs::CbsStateApp;
pub(crate) use crate::networks::NetworksApp;
pub(crate) use crate::networks::window::NetworksWindowApp;
pub(crate) use crate::subnetworks_window::SubnetworksWindowApp;
