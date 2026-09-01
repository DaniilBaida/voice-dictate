//! Background reachability check for the local speech server.
//!
//! The tray thread must never block, so a worker polls and publishes the result
//! for the menu to read. A TCP connect is enough to separate the failure that
//! matters, a server that is not listening, from a working setup.

use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

const INTERVAL: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Spawns the poller and returns the flag it keeps up to date.
pub fn spawn(server_url: &str) -> Arc<AtomicBool> {
    let reachable = Arc::new(AtomicBool::new(true));
    let Some(address) = authority(server_url) else {
        tracing::warn!("could not read a host:port out of {server_url}");
        return reachable;
    };

    let flag = Arc::clone(&reachable);
    std::thread::Builder::new()
        .name("health".into())
        .spawn(move || loop {
            flag.store(is_listening(&address), Ordering::Relaxed);
            std::thread::sleep(INTERVAL);
        })
        .ok();

    reachable
}

fn is_listening(address: &str) -> bool {
    let Ok(addresses) = address.to_socket_addrs() else {
        return false;
    };
    addresses.into_iter().any(|addr| {
        TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok()
    })
}

/// Pulls `host:port` out of a base URL, defaulting the port from the scheme.
fn authority(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host_port = rest.split('/').next()?;
    if host_port.is_empty() {
        return None;
    }
    if host_port.rsplit(':').next()?.parse::<u16>().is_ok() {
        return Some(host_port.to_string());
    }
    let port = if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    };
    Some(format!("{host_port}:{port}"))
}

#[cfg(test)]
mod tests {
    use super::authority;

    #[test]
    fn reads_host_and_port_from_a_base_url() {
        assert_eq!(
            authority("http://127.0.0.1:8080/v1").as_deref(),
            Some("127.0.0.1:8080")
        );
        assert_eq!(
            authority("https://example.com/v1").as_deref(),
            Some("example.com:443")
        );
        assert_eq!(
            authority("http://example.com").as_deref(),
            Some("example.com:80")
        );
        assert_eq!(authority("127.0.0.1:8080"), None);
    }
}
