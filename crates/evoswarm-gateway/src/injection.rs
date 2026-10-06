//! Exemplar injection seam and its opt-out predicate (e4-4, AD-2).
//!
//! e4-3 (exemplar injection over FalkorDB) is BLOCKED on e2-5, but the *contract* it will
//! implement and the *bypass* this story owns do not depend on it: the proxy holds an
//! optional injector behind this trait, and this module decides — from the request headers
//! alone — whether that injector may be consulted at all.
//!
//! The story invariant (§2) is that opting out "bypasses memory lookup entirely". That is a
//! performance and privacy claim, not just an output claim: a discarded injection still did a
//! memory lookup. So the predicate here is used to gate the *call*, never to filter the
//! result. See `ProxyState`/`forward` for the 0-call fast path.

use async_trait::async_trait;
use axum::http::HeaderMap;

/// Request header a client sets to steer injection for a single request.
pub const INJECT_HEADER: &str = "x-evoswarm-inject";

/// The only header value that opts out. Matched case-insensitively.
pub const INJECT_OFF_VALUE: &str = "off";

/// A source of extra context to merge into an outgoing request body.
///
/// Takes the raw request bytes and returns replacement bytes, or `None` to leave the request
/// untouched (no relevant exemplars, a lookup timeout, a body this cannot rewrite). e4-3's
/// FalkorDB-backed injector will implement this; tests inject a counting stub to prove the
/// opt-out path never calls it.
#[async_trait]
pub trait ExemplarInjector: Send + Sync {
    async fn inject(&self, body: &[u8]) -> Option<Vec<u8>>;
}

/// True when the caller opted this request out via `x-evoswarm-inject: off`.
///
/// Deliberately narrow: anything that is not exactly `off` (ignoring ASCII case) leaves
/// injection on, including a missing header, an empty value, a near-miss like
/// `OFF-but-not-really`, and a non-UTF-8 value we cannot parse. An unparseable opt-out is
/// treated as no opt-out, so a client can never *accidentally* disable injection with a
/// malformed header — and a malformed header can neither be used to probe the gateway.
pub fn header_opts_out(headers: &HeaderMap) -> bool {
    headers
        .get(INJECT_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(INJECT_OFF_VALUE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers_with(value: HeaderValue) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static(INJECT_HEADER), value);
        h
    }

    /// `off` opts out in any ASCII case; everything else does not.
    #[test]
    fn only_the_off_value_opts_out() {
        assert!(header_opts_out(&headers_with(HeaderValue::from_static(
            "off"
        ))));
        assert!(header_opts_out(&headers_with(HeaderValue::from_static(
            "OFF"
        ))));
        assert!(header_opts_out(&headers_with(HeaderValue::from_static(
            "OfF"
        ))));

        // Non-matching values keep injection on...
        assert!(!header_opts_out(&headers_with(HeaderValue::from_static(
            "on"
        ))));
        // ...including near-misses that merely start with `off`...
        assert!(!header_opts_out(&headers_with(HeaderValue::from_static(
            "OFF-but-not-really"
        ))));
        // ...and the empty value (a client that sent the header with no value has not asked
        // for anything).
        assert!(!header_opts_out(&headers_with(HeaderValue::from_static(
            ""
        ))));
    }

    /// A missing header, and a header whose bytes are not valid UTF-8, both leave injection
    /// on — the parse failure must not be read as an opt-out.
    #[test]
    fn missing_and_non_utf8_headers_do_not_opt_out() {
        assert!(
            !header_opts_out(&HeaderMap::new()),
            "no header means no opt-out"
        );
        // 0xFF is legal in a header value but is not valid UTF-8, so `to_str` fails.
        let raw = HeaderValue::from_bytes(&[0xFF, 0xFE]).expect("obs-text header value");
        assert!(
            raw.to_str().is_err(),
            "precondition: value is not UTF-8 decodable"
        );
        assert!(
            !header_opts_out(&headers_with(raw)),
            "an undecodable value must not be treated as an opt-out"
        );
    }
}
