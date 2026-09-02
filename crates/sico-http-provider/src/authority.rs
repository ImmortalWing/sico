//! Strict URL/authority canonicalization and address policy (M12
//! STEP-0113): one parser shared by policy and transport. Every confusion
//! class in RFC-0037 §2/§4 is a typed refusal; canonicalization happens
//! exactly once and both spellings of one authority collapse to it.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Maximum canonical authority length (RFC-0037 bounds: URL ≤ 8 KiB).
pub const MAX_AUTHORITY_BYTES: usize = 320;

/// A canonicalized endpoint: scheme, canonical ASCII host and effective
/// port. Equality is grant identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    /// Grant identity string: `scheme + canonical host + effective port`.
    #[must_use]
    pub fn identity(&self) -> String {
        format!("{}|{}|{}", self.scheme, self.host, self.port)
    }
}

/// Schemes accepted in v1; anything else is a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    Http,
    Https,
    /// Development-only scheme for literal private/loopback endpoints.
    HttpPrivate,
    HttpsPrivate,
}

impl Scheme {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            "http+private" => Some(Self::HttpPrivate),
            "https+private" => Some(Self::HttpsPrivate),
            _ => None,
        }
    }

    #[must_use]
    pub fn default_port(self) -> u16 {
        match self {
            Self::Http | Self::HttpPrivate => 80,
            Self::Https | Self::HttpsPrivate => 443,
        }
    }

    #[must_use]
    pub fn is_tls(self) -> bool {
        matches!(self, Self::Https | Self::HttpsPrivate)
    }
}

impl core::fmt::Display for Scheme {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::Http => "http",
            Self::Https => "https",
            Self::HttpPrivate => "http+private",
            Self::HttpsPrivate => "https+private",
        };
        f.write_str(text)
    }
}

/// Why an authority was refused; stable names, part of the contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    Oversize,
    UnknownScheme,
    NotCanonicalIpv4,
    Ipv4MappedIpv6,
    ZoneId,
    NonAsciiHost,
    EmptyHost,
    HostTooLong,
    BadDnsLabel,
    UppercaseHost,
    PercentEncoding,
    UserInfo,
    Fragment,
    BadPort,
    AmbiguousForm,
}

impl core::fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::Oversize => "authority oversize",
            Self::UnknownScheme => "unknown scheme",
            Self::NotCanonicalIpv4 => "IPv4 literal is not canonical",
            Self::Ipv4MappedIpv6 => "IPv4-mapped IPv6 refused",
            Self::ZoneId => "IPv6 zone/scope id refused",
            Self::NonAsciiHost => "non-ASCII host refused (IDNA not accepted in grants)",
            Self::EmptyHost => "empty host",
            Self::HostTooLong => "DNS host too long",
            Self::BadDnsLabel => "DNS label is not canonical LDH",
            Self::UppercaseHost => "uppercase host refused",
            Self::PercentEncoding => "percent-encoded host refused",
            Self::UserInfo => "user-info in authority refused",
            Self::Fragment => "fragment refused",
            Self::BadPort => "port is not a canonical decimal in 1..=65535",
            Self::AmbiguousForm => "ambiguous authority form refused",
        };
        f.write_str(name)
    }
}

impl std::error::Error for AuthorityError {}

/// Canonicalizes `url` (`<scheme://host[:port][/path]>`) into an
/// [`Endpoint`]. The host part must already be a canonical DNS (LDH,
/// punycode is just ASCII) or a canonical IP literal — the contract is
/// that grants and URLs share this exact parser, so non-canonical
/// spellings are refused rather than silently mapped.
///
/// # Errors
///
/// Returns [`AuthorityError`] for every non-canonical or oversized form;
/// see the variant list for the stable refusal classes.
pub fn canonicalize_url(url: &str) -> Result<Endpoint, AuthorityError> {
    if url.len() > 8 * 1024 {
        return Err(AuthorityError::Oversize);
    }
    let (scheme_text, rest) = url.split_once("://").ok_or(AuthorityError::UnknownScheme)?;
    let scheme = Scheme::parse(scheme_text).ok_or(AuthorityError::UnknownScheme)?;
    let authority_and_path = rest;
    if authority_and_path.contains('#') {
        return Err(AuthorityError::Fragment);
    }
    if authority_and_path.contains('@') {
        return Err(AuthorityError::UserInfo);
    }
    let authority = authority_and_path
        .split(['/', '?'])
        .next()
        .unwrap_or_default();
    if authority.is_empty() {
        return Err(AuthorityError::EmptyHost);
    }
    if authority.len() > MAX_AUTHORITY_BYTES {
        return Err(AuthorityError::Oversize);
    }
    canonicalize_authority(&scheme, authority)
}

/// Canonicalizes a bare `host[:port]` authority under `scheme`.
///
/// # Errors
///
/// Same [`AuthorityError`] classes as [`canonicalize_url`].
pub fn canonicalize_authority(
    scheme: &Scheme,
    authority: &str,
) -> Result<Endpoint, AuthorityError> {
    if authority.len() > MAX_AUTHORITY_BYTES {
        return Err(AuthorityError::Oversize);
    }
    if authority.contains('%') {
        return Err(AuthorityError::PercentEncoding);
    }
    // Bracketed IPv6 literal.
    if let Some(rest) = authority.strip_prefix('[') {
        let Some(close) = rest.find(']') else {
            return Err(AuthorityError::AmbiguousForm);
        };
        let host_text = &rest[..close];
        let port_text = rest[close + 1..].strip_prefix(':');
        let host = canonical_ipv6(host_text)?;
        let port = port_from(port_text, scheme.default_port())?;
        return Ok(Endpoint {
            scheme: *scheme,
            host,
            port,
        });
    }
    if authority.contains('[') || authority.contains(']') {
        return Err(AuthorityError::AmbiguousForm);
    }
    let (host_text, port_text) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if host_text.is_empty() {
        return Err(AuthorityError::EmptyHost);
    }
    let host = canonical_host(host_text)?;
    let port = port_from(port_text, scheme.default_port())?;
    Ok(Endpoint {
        scheme: *scheme,
        host,
        port,
    })
}

fn port_from(port_text: Option<&str>, default: u16) -> Result<u16, AuthorityError> {
    match port_text {
        None => Ok(default),
        Some(text) => {
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(AuthorityError::BadPort);
            }
            if text.len() > 1 && text.starts_with('0') {
                return Err(AuthorityError::BadPort);
            }
            let port: u16 = text.parse().map_err(|_| AuthorityError::BadPort)?;
            if port == 0 {
                return Err(AuthorityError::BadPort);
            }
            Ok(port)
        }
    }
}

fn canonical_ipv6(text: &str) -> Result<String, AuthorityError> {
    if text.contains('%') {
        return Err(AuthorityError::ZoneId);
    }
    let address: Ipv6Addr = text.parse().map_err(|_| AuthorityError::AmbiguousForm)?;
    if address.to_ipv4_mapped().is_some() {
        return Err(AuthorityError::Ipv4MappedIpv6);
    }
    Ok(address.to_string())
}

/// Canonicalizes a non-bracketed host: canonical IPv4 literal, or a
/// canonical LDH DNS name. Non-ASCII, uppercase and non-canonical IPv4
/// are refusals — never silent mappings.
fn canonical_host(text: &str) -> Result<String, AuthorityError> {
    if text.is_empty() {
        return Err(AuthorityError::EmptyHost);
    }
    if text.len() > 253 {
        return Err(AuthorityError::HostTooLong);
    }
    if !text.is_ascii() {
        return Err(AuthorityError::NonAsciiHost);
    }
    if text.bytes().any(|b| b.is_ascii_uppercase()) {
        return Err(AuthorityError::UppercaseHost);
    }
    // Dotted-quad-looking hosts must be canonical IPv4 literals; a label
    // with a hex prefix (`0x…`) is a textual-IPv4 trick, never a hostname.
    let quad_like = text.contains('.')
        && text.split('.').all(|label| {
            !label.is_empty()
                && label.bytes().all(|b| b.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() || b == b'x' || b == b'X')
        });
    if quad_like {
        if text.split('.').any(|label| {
            label.len() > 1
                && (label.starts_with('0') || label.starts_with("0x") || label.starts_with("0X"))
        }) {
            return Err(AuthorityError::NotCanonicalIpv4);
        }
        if text
            .split('.')
            .any(|label| label.starts_with("0x") || label.starts_with("0X"))
        {
            return Err(AuthorityError::NotCanonicalIpv4);
        }
        let address: Ipv4Addr = text.parse().map_err(|_| AuthorityError::NotCanonicalIpv4)?;
        if address.to_string() != text {
            return Err(AuthorityError::NotCanonicalIpv4);
        }
        return Ok(text.to_owned());
    }
    // A bare number is an ambiguous form (could be decimal/hex IPv4).
    if !text.contains('.') && text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(AuthorityError::AmbiguousForm);
    }
    if text.starts_with('.')
        || text.ends_with('.')
        || text.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err(AuthorityError::BadDnsLabel);
    }
    Ok(text.to_owned())
}

/// Address classification for RFC-0037 §4: DNS hostnames may not resolve
/// into these ranges without an explicit private-network grant, and
/// literal private endpoints require the `+private` scheme spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressClass {
    Public,
    Private,
    Loopback,
    LinkLocal,
    Unspecified,
}

#[must_use]
pub fn classify(address: IpAddr) -> AddressClass {
    match address {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                AddressClass::Loopback
            } else if v6.is_unspecified() {
                AddressClass::Unspecified
            } else if v6.segments()[0] & 0xfe00 == 0xfc00 || v6.segments()[0] & 0xffc0 == 0xfe80 {
                AddressClass::Private
            } else {
                AddressClass::Public
            }
        }
    }
}

fn classify_v4(v4: Ipv4Addr) -> AddressClass {
    let o = v4.octets();
    match o {
        [127, ..] => AddressClass::Loopback,
        [0, 0, 0, 0] => AddressClass::Unspecified,
        [169, 254, ..] => AddressClass::LinkLocal,
        [10, ..] | [192, 168, ..] | [172, 16..=31, ..] | [100, 64..=127, ..] => {
            AddressClass::Private
        }
        [198, 18..=19, ..] => AddressClass::Private,
        _ => AddressClass::Public,
    }
}

/// Address-policy gate (RFC-0037 §4): plain schemes refuse all
/// private/special ranges; the development `+private` schemes accept
/// them. Called per connection attempt, per redirect target and per pool
/// reuse — never cached across uses (DNS rebinding defense).
///
/// # Errors
///
/// [`AddressError::PrivateNetworkDenied`] when the scheme does not allow
/// the address class.
pub fn address_allowed_for_scheme(scheme: Scheme, address: IpAddr) -> Result<(), AddressError> {
    match (scheme, classify(address)) {
        (Scheme::Http | Scheme::Https, AddressClass::Public) => Ok(()),
        (
            Scheme::HttpPrivate | Scheme::HttpsPrivate,
            AddressClass::Public
            | AddressClass::Private
            | AddressClass::Loopback
            | AddressClass::LinkLocal,
        ) => Ok(()),
        _ => Err(AddressError::PrivateNetworkDenied),
    }
}

/// Stable policy-failure names (RFC-0037 §1 error taxonomy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    PrivateNetworkDenied,
}

impl core::fmt::Display for AddressError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("private/special network address denied for this scheme")
    }
}

impl std::error::Error for AddressError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(url: &str) -> Endpoint {
        canonicalize_url(url).expect("canonical")
    }

    #[test]
    fn canonical_forms_roundtrip_and_default_ports_collapse() {
        let e = endpoint("https://api.example.com");
        assert_eq!(e.identity(), "https|api.example.com|443");
        let e = endpoint("https://api.example.com:443/path");
        assert_eq!(e.identity(), "https|api.example.com|443");
        let e = endpoint("http://api.example.com:8080");
        assert_eq!(e.identity(), "http|api.example.com|8080");
        let e = endpoint("https://[2001:db8::1]:8443");
        assert_eq!(e.identity(), "https|2001:db8::1|8443");
        // Canonical IPv4 literal.
        let e = endpoint("https://192.0.2.10");
        assert_eq!(e.identity(), "https|192.0.2.10|443");
    }

    #[test]
    fn ipv4_confusion_tricks_are_refused() {
        for url in [
            "https://0300.0000.0002.0250",  // octal
            "https://0xc0.0x00.0x02.0x90",  // hex
            "https://3232235570",           // bare decimal
            "https://192.000.000.002",      // leading zeros
            "https://192.0.2.10.",          // trailing dot
            "https://[::ffff:192.0.2.128]", // IPv4-mapped IPv6
            "https://[fe80::1%25eth0]",     // zone id
            "https://[2001:db8::1]:000443", // padded port
            "https://[2001:db8::1]:0",      // zero port
        ] {
            assert!(canonicalize_url(url).is_err(), "{url} must be refused");
        }
    }

    #[test]
    fn idna_and_case_are_refused_not_translated() {
        // Non-ASCII: refused (grants and URLs are canonical-ASCII/punycode).
        assert!(matches!(
            canonicalize_url("https://bücher.example.com"),
            Err(AuthorityError::NonAsciiHost)
        ));
        // Uppercase is refused, not lowercased: grants never silently map.
        assert!(matches!(
            canonicalize_url("https://API.example.com"),
            Err(AuthorityError::UppercaseHost)
        ));
        // Percent-encoded host refused.
        assert!(matches!(
            canonicalize_url("https://api%2eexample.com"),
            Err(AuthorityError::PercentEncoding)
        ));
    }

    #[test]
    fn userinfo_fragment_and_schemes_are_refused() {
        assert!(matches!(
            canonicalize_url("https://user:pass@api.example.com"),
            Err(AuthorityError::UserInfo)
        ));
        assert!(matches!(
            canonicalize_url("https://api.example.com/#frag"),
            Err(AuthorityError::Fragment)
        ));
        assert!(matches!(
            canonicalize_url("ftp://api.example.com"),
            Err(AuthorityError::UnknownScheme)
        ));
        assert!(matches!(
            canonicalize_url("https://[bad::ipv6"),
            Err(AuthorityError::AmbiguousForm)
        ));
    }

    #[test]
    fn private_ranges_classify_correctly() {
        use std::net::IpAddr::{V4, V6};
        assert_eq!(
            classify(V4("127.0.0.1".parse().unwrap())),
            AddressClass::Loopback
        );
        assert_eq!(
            classify(V4("10.0.0.5".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V4("192.168.1.1".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V4("172.16.0.9".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V4("100.64.0.1".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V4("169.254.1.2".parse().unwrap())),
            AddressClass::LinkLocal
        );
        assert_eq!(
            classify(V4("0.0.0.0".parse().unwrap())),
            AddressClass::Unspecified
        );
        assert_eq!(
            classify(V4("93.184.216.34".parse().unwrap())),
            AddressClass::Public
        );
        assert_eq!(classify(V6("::1".parse().unwrap())), AddressClass::Loopback);
        assert_eq!(
            classify(V6("fe80::1".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V6("fc00::1".parse().unwrap())),
            AddressClass::Private
        );
        assert_eq!(
            classify(V6("2001:db8::1".parse().unwrap())),
            AddressClass::Public
        );
    }

    #[test]
    fn plus_private_schemes_carry_the_dev_grant_semantics() {
        let e = endpoint("http+private://127.0.0.1:9000");
        assert_eq!(e.scheme, Scheme::HttpPrivate);
        assert!(!e.scheme.is_tls());
        let e = endpoint("https+private://localhost");
        assert!(e.scheme.is_tls());
        assert_eq!(e.port, 443);
    }
}

#[cfg(test)]
mod step0113_tests {
    use super::*;

    #[test]
    fn punycode_labels_are_just_ascii_dns() {
        // The IDNA policy: grants and URLs speak canonical ASCII only.
        // Punycode (`xn--…`) is a valid LDH label and passes; Unicode is
        // refused rather than converted — conversion ambiguity can never
        // bypass a grant because both sides share this parser.
        let e = canonicalize_url("https://xn--bcher-kva.example.com").expect("punycode passes");
        assert_eq!(e.identity(), "https|xn--bcher-kva.example.com|443");
        assert!(matches!(
            canonicalize_url("https://bücher.example.com"),
            Err(AuthorityError::NonAsciiHost)
        ));
        // Homoglyph trick via percent-encoding refused outright.
        assert!(matches!(
            canonicalize_url("https://%C3%BC.example.com"),
            Err(AuthorityError::PercentEncoding)
        ));
    }

    #[test]
    fn dns_pin_policy_revalidates_every_use() {
        // The pinning contract: a resolved address is classified at each
        // use. A DNS name resolving into a denied range must be refused
        // before connect even when the grant was created while the name
        // pointed at a public address (rebinding defense is per-use).
        let public: IpAddr = "93.184.216.34".parse().unwrap();
        let rebound: IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(classify(public), AddressClass::Public);
        assert_eq!(classify(rebound), AddressClass::Loopback);
        // The policy function refuses loopback for a plain (non-private)
        // scheme.
        assert!(address_allowed_for_scheme(Scheme::Https, rebound).is_err());
        // Private-scheme grants (development) accept loopback literals.
        assert!(address_allowed_for_scheme(Scheme::HttpPrivate, rebound).is_ok());
        // Public addresses pass for normal schemes.
        assert!(address_allowed_for_scheme(Scheme::Https, public).is_ok());
    }

    #[test]
    fn literal_private_endpoints_require_private_schemes() {
        assert!(address_allowed_for_scheme(Scheme::Https, "10.0.0.1".parse().unwrap()).is_err());
        assert!(address_allowed_for_scheme(Scheme::Https, "192.168.0.1".parse().unwrap()).is_err());
        assert!(
            address_allowed_for_scheme(Scheme::HttpPrivate, "127.0.0.1".parse().unwrap()).is_ok()
        );
        assert!(
            address_allowed_for_scheme(Scheme::HttpsPrivate, "127.0.0.1".parse().unwrap()).is_ok()
        );
    }
}
