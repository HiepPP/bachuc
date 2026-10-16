//! Host form validation. A host is a name, a plain `ws://` URL, and an optional password
//! (ADR-0040): `wss`, the relay, and SSH are out.

use bachuc_client::HostConfig;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostError {
    NameRequired,
    UrlRequired,
    /// The URL does not start with `ws://`.
    NotWs,
    /// No address after `ws://`, or a port outside 1 to 65535, or a space in the URL.
    InvalidUrl,
    DuplicateName,
    DuplicateUrl,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            HostError::NameRequired => "Name is required",
            HostError::UrlRequired => "URL is required",
            HostError::NotWs => "Use a plain ws:// URL, for example ws://127.0.0.1:6768/ws",
            HostError::InvalidUrl => "Invalid URL. Check the address and the port (1 to 65535)",
            HostError::DuplicateName => "A host with this name exists",
            HostError::DuplicateUrl => "A host with this URL exists",
        })
    }
}

impl std::error::Error for HostError {}

/// Checks the form fields and builds the host. `editing` is the index of the host being edited,
/// which the duplicate checks skip. A URL with no path gets `/ws`, the daemon's socket path.
pub fn validate_host(
    name: &str,
    url: &str,
    password: &str,
    hosts: &[HostConfig],
    editing: Option<usize>,
) -> Result<HostConfig, HostError> {
    let name = name.trim();
    let url = url.trim();
    if name.is_empty() {
        return Err(HostError::NameRequired);
    }
    if url.is_empty() {
        return Err(HostError::UrlRequired);
    }
    let scheme_end = url.find("://").ok_or(HostError::NotWs)?;
    if !url[..scheme_end].eq_ignore_ascii_case("ws") {
        return Err(HostError::NotWs);
    }
    let rest = &url[scheme_end + 3..];
    if rest.chars().any(char::is_whitespace) {
        return Err(HostError::InvalidUrl);
    }
    let (authority, path) = match rest.find(['/', '?', '#']) {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, ""),
    };
    let (address, port) = match authority.rsplit_once(':') {
        // A bracketed IPv6 address has colons of its own; only a port after `]` counts.
        Some((address, port)) if !port.contains(']') => (address, Some(port)),
        _ => (authority, None),
    };
    if address.is_empty() || address == "[" {
        return Err(HostError::InvalidUrl);
    }
    if let Some(port) = port {
        match port.parse::<u32>() {
            Ok(1..=65535) => {}
            _ => return Err(HostError::InvalidUrl),
        }
    }
    let url = if path.is_empty() {
        format!(
            "{}://{authority}/ws",
            &url[..scheme_end].to_ascii_lowercase()
        )
    } else {
        format!("{}://{rest}", &url[..scheme_end].to_ascii_lowercase())
    };
    for (index, host) in hosts.iter().enumerate() {
        if Some(index) == editing {
            continue;
        }
        if host.name.trim().eq_ignore_ascii_case(name) {
            return Err(HostError::DuplicateName);
        }
        if host.url == url {
            return Err(HostError::DuplicateUrl);
        }
    }
    let password = password.trim();
    Ok(HostConfig {
        name: name.to_string(),
        url,
        password: (!password.is_empty()).then(|| password.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(name: &str, url: &str) -> HostConfig {
        HostConfig {
            name: name.into(),
            url: url.into(),
            password: None,
        }
    }

    fn existing() -> Vec<HostConfig> {
        vec![
            host("release", "ws://127.0.0.1:6767/ws"),
            host("live", "ws://127.0.0.1:6768/ws"),
        ]
    }

    #[test]
    fn accepts_a_ws_host_with_a_password() {
        let built = validate_host(
            " macmini ",
            " ws://127.0.0.1:6769/ws ",
            " pw ",
            &existing(),
            None,
        )
        .unwrap();
        assert_eq!(built.name, "macmini");
        assert_eq!(built.url, "ws://127.0.0.1:6769/ws");
        assert_eq!(built.password.as_deref(), Some("pw"));
    }

    #[test]
    fn empty_password_is_none_and_a_bare_address_gets_the_socket_path() {
        let built =
            validate_host("tunnel", "ws://localhost:6770", "  ", &existing(), None).unwrap();
        assert_eq!(built.url, "ws://localhost:6770/ws");
        assert_eq!(built.password, None);
    }

    #[test]
    fn rejects_empty_fields() {
        assert_eq!(
            validate_host(" ", "ws://a:1/ws", "", &[], None),
            Err(HostError::NameRequired)
        );
        assert_eq!(
            validate_host("a", "  ", "", &[], None),
            Err(HostError::UrlRequired)
        );
    }

    #[test]
    fn only_plain_ws_is_allowed() {
        for url in [
            "wss://example.com/ws",
            "http://example.com",
            "127.0.0.1:6768",
            "relay://x",
        ] {
            assert_eq!(
                validate_host("a", url, "", &[], None),
                Err(HostError::NotWs),
                "{url}"
            );
        }
    }

    #[test]
    fn rejects_a_bad_address_or_port() {
        for url in [
            "ws://",
            "ws:///ws",
            "ws://host:0/ws",
            "ws://host:65536/ws",
            "ws://host:abc/ws",
            "ws://ho st:1/ws",
            "ws://:6768/ws",
        ] {
            assert_eq!(
                validate_host("a", url, "", &[], None),
                Err(HostError::InvalidUrl),
                "{url}"
            );
        }
        assert!(validate_host("a", "ws://[::1]:6768/ws", "", &[], None).is_ok());
    }

    #[test]
    fn duplicate_name_or_url_is_refused_except_for_the_edited_host() {
        let hosts = existing();
        assert_eq!(
            validate_host("LIVE", "ws://10.0.0.1:1/ws", "", &hosts, None),
            Err(HostError::DuplicateName)
        );
        assert_eq!(
            validate_host("other", "ws://127.0.0.1:6768/ws", "", &hosts, None),
            Err(HostError::DuplicateUrl)
        );
        // Editing `live` may keep its own name and URL.
        assert!(validate_host("live", "ws://127.0.0.1:6768/ws", "pw", &hosts, Some(1)).is_ok());
        // But not take the URL of `release`.
        assert_eq!(
            validate_host("live", "ws://127.0.0.1:6767/ws", "", &hosts, Some(1)),
            Err(HostError::DuplicateUrl)
        );
    }
}
