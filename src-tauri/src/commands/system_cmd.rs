use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Error;

/// A local network interface the user can bind a listener to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    pub name: String,
    pub address: String,
}

/// List the machine's IPv4 interface addresses (for the bind-address picker).
#[tauri::command]
pub fn list_network_interfaces() -> Result<Vec<NetworkInterface>, Error> {
    let mut interfaces: Vec<NetworkInterface> = if_addrs::get_if_addrs()
        .map_err(Error::Io)?
        .into_iter()
        .filter(|iface| matches!(iface.ip(), IpAddr::V4(_)))
        .map(|iface| NetworkInterface {
            address: iface.ip().to_string(),
            name: iface.name,
        })
        .collect();

    interfaces.sort_by(|a, b| a.address.cmp(&b.address));
    interfaces.dedup_by(|a, b| a.address == b.address);
    Ok(interfaces)
}
