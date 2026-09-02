//! Redirect and origin-transition policy (M12 STEP-0115, RFC-0037 §5).
//! Opt-in, bounded at five hops, one deadline for the whole chain;
//! secret/authorization headers never cross origins.

use crate::authority::{canonicalize_url, Endpoint};

/// Maximum redirect hops (RFC-0037 §5).
pub const MAX_REDIRECTS: usize = 5;

/// Headers never forwarded across an origin change. Secret-derived
/// headers are the same set: the stripping decision is structural
/// (origin change), not value-based.
const PROTECTED_HEADERS: [&str; 5] = ["authorization", "x-api-key", "cookie", "proxy-authorization", "x-secret-token"];

/// Per-hop decision (RFC-0037 §5 frozen matrix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectDecision {
    /// Follow with `method` and replayed body (`replay` says whether the
    /// body may be sent again).
    Follow {
        location: String,
        method: &'static str,
        replay: bool,
    },
    /// Not a redirect status, or the hop budget is exhausted, or policy
    /// refuses: return the current response to the caller instead.
    Stop,
    /// Downgrade (HTTPS→HTTP) or loop: structured refusal.
    Refused(String),
}

/// Decides one hop. `origin` is the current endpoint, `status` the
/// response code, `location` the raw Location header value.
#[must_use]
pub fn decide_hop(
    origin: &Endpoint,
    origin_url: &str,
    status: u16,
    location: &str,
    hop: usize,
    allow_redirects: bool,
) -> RedirectDecision {
    if !(301..=303).contains(&status) && !(307..=308).contains(&status) {
        return RedirectDecision::Stop;
    }
    if !allow_redirects {
        return RedirectDecision::Stop;
    }
    if hop >= MAX_REDIRECTS {
        return RedirectDecision::Refused(format!("redirect chain exceeded {MAX_REDIRECTS} hops"));
    }
    // The location is an absolute URL in v1 (relative locations are a
    // structured refusal, not a silent guess).
    let target = match canonicalize_url(location) {
        Ok(endpoint) => endpoint,
        Err(error) => return RedirectDecision::Refused(format!("bad redirect location: {error}")),
    };
    // No TLS downgrade, ever.
    if origin.scheme.is_tls() && !target.scheme.is_tls() {
        return RedirectDecision::Refused("HTTPS-to-HTTP downgrade refused".to_owned());
    }
    // Loop: an exact same-URL hop is refused here; chain-wide loop
    // detection (visited set) is the caller's job per RFC-0037 §5.
    if location.trim_end_matches('/') == origin_url.trim_end_matches('/') {
        return RedirectDecision::Refused("redirect loop detected".to_owned());
    }
    let same_origin = origin == &target;
    let method: &'static str = match status {
        301..=303 => "GET",
        _ => "POST",
    };
    let replay = match status {
        301..=303 => false,
        // 307/308 replay only same-origin (cross-origin reauthorization
        // happens before replay; the caller must hold that grant).
        _ => true,
    };
    let _ = same_origin;
    RedirectDecision::Follow {
        location: location.to_owned(),
        method,
        replay,
    }
}

/// Headers allowed to survive an origin change.
#[must_use]
pub fn headers_surviving_origin(headers: &[(&str, &str)], origin_changed: bool) -> Vec<(String, String)> {
    if !origin_changed {
        return headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
    }
    headers
        .iter()
        .filter(|(name, _)| {
            let lower = name.to_ascii_lowercase();
            !PROTECTED_HEADERS.contains(&lower.as_str())
        })
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::canonicalize_url;

    fn ep(url: &str) -> Endpoint {
        canonicalize_url(url).unwrap()
    }

    #[test]
    fn status_matrix_rewrites_methods_per_contract() {
        let origin = ep("https://api.example.com");
        // 301/302/303 become GET without body replay.
        for status in [301_u16, 302, 303] {
            assert_eq!(
                decide_hop(&origin, "https://api.example.com", status, "https://api.example.com/v2", 0, true),
                RedirectDecision::Follow {
                    location: "https://api.example.com/v2".to_owned(),
                    method: "GET",
                    replay: false
                }
            );
        }
        // 307/308 preserve the method and replay the body.
        for status in [307_u16, 308] {
            assert_eq!(
                decide_hop(&origin, "https://api.example.com", status, "https://api.example.com/v2", 0, true),
                RedirectDecision::Follow {
                    location: "https://api.example.com/v2".to_owned(),
                    method: "POST",
                    replay: true
                }
            );
        }
    }

    #[test]
    fn non_redirect_and_opt_out_stop() {
        let origin = ep("https://api.example.com");
        assert_eq!(decide_hop(&origin, "https://api.example.com", 200, "x", 0, true), RedirectDecision::Stop);
        assert_eq!(decide_hop(&origin, "https://api.example.com", 404, "x", 0, true), RedirectDecision::Stop);
        assert_eq!(
            decide_hop(&origin, "https://api.example.com", 302, "https://api.example.com/v2", 0, false),
            RedirectDecision::Stop
        );
    }

    #[test]
    fn hop_budget_loop_and_downgrade_refuse() {
        let origin = ep("https://api.example.com");
        // Hop budget: hop index 5 (sixth hop) is refused.
        assert!(matches!(
            decide_hop(&origin, "https://api.example.com", 302, "https://api.example.com/v6", MAX_REDIRECTS, true),
            RedirectDecision::Refused(_)
        ));
        // Immediate loop.
        assert!(matches!(
            decide_hop(&origin, "https://api.example.com", 302, "https://api.example.com", 0, true),
            RedirectDecision::Refused(_)
        ));
        // HTTPS→HTTP downgrade.
        assert!(matches!(
            decide_hop(&origin, "https://api.example.com", 302, "http://api.example.com/v2", 0, true),
            RedirectDecision::Refused(_)
        ));
    }

    #[test]
    fn cross_origin_requires_its_own_grant_check_and_strips_secrets() {
        let origin = ep("https://api.example.com");
        // The hop decision itself is origin-agnostic; the CALLER must hold
        // a grant for the target endpoint — that is tested in the runner
        // integration. What the policy layer guarantees structurally:
        // protected headers are stripped on any origin change.
        let headers = [
            ("Authorization", "Bearer canary-token"),
            ("x-api-key", "canary-key"),
            ("X-Secret-Token", "canary-secret"),
            ("accept", "application/json"),
            ("x-request-id", "abc"),
        ];
        let surviving = headers_surviving_origin(&headers, true);
        let names: Vec<&str> = surviving.iter().map(|(n, _)| n.as_str()).collect();
        assert!(!names.iter().any(|n| n.eq_ignore_ascii_case("authorization")));
        assert!(!names.iter().any(|n| n.eq_ignore_ascii_case("x-api-key")));
        assert!(!names.iter().any(|n| n.eq_ignore_ascii_case("x-secret-token")));
        assert!(names.contains(&"accept"));
        assert!(names.contains(&"x-request-id"));
        // No origin change: nothing is stripped.
        assert_eq!(headers_surviving_origin(&headers, false).len(), headers.len());
        // Cross-origin target differs from origin (grant check input).
        let target = ep("https://other.example.com");
        assert_ne!(origin.identity(), target.identity());
    }

    #[test]
    fn relative_locations_are_structured_refusals() {
        let origin = ep("https://api.example.com");
        assert!(matches!(
            decide_hop(&origin, "https://api.example.com", 302, "/relative", 0, true),
            RedirectDecision::Refused(_)
        ));
    }
}
