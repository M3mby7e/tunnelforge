//! Per-mode forwarding: local (`-L`), remote (`-R`), and dynamic (`-D`, SOCKS5).

pub mod dynamic;
pub mod local;
pub mod remote;
