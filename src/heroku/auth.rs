//! Helpers around Heroku's use of a shared secret to authenticate webhook
//! requests.
//!
//! The secret is sourced from `$HEROKU_SECRET`. If this value changes then
//! any previously defined webhooks will be invalid and their requests will
//! fail. Any arbitrary text can act as a secret.
//!
//! The secret is shared with Heroku when initialising the webhook, which Heroku
//! then use to sign each request body, the result of which is included in a
//! header. We compare our own signature against it to know if the request
//! really came from Heroku.
//!
//! <https://devcenter.heroku.com/articles/app-webhooks#using-the-shared-secret>

use axum::http::header::HeaderMap;
use base64::{engine::general_purpose::STANDARD as b64, Engine};
use hmac::{Hmac, Mac};
use hyper::body::Bytes;
use sha2::Sha256;

/// A newtype wrapper around the Heroku secret.
#[derive(Clone)]
pub struct HerokuSecret(pub String);

/// What can go wrong when validating a request's secret.
pub enum SecretError {
    Missing,
    Invalid,
}

/// Test a request's headers for a valid secret-based signature.
///
/// The payload body should be supplied entirely unmodified from the request.
///
/// Requests which fail this predicate, or which don't have a signature at all,
/// should be considered unauthenticated.
pub async fn validate_request_signature(
    secret: &HerokuSecret,
    body: &Bytes,
    headers: &HeaderMap,
) -> Result<(), SecretError> {
    match headers.get("Heroku-Webhook-Hmac-SHA256") {
        None => Err(SecretError::Missing),
        Some(h) => match h.to_str() {
            Err(_) => Err(SecretError::Invalid),
            Ok(v) => match is_valid_signature(secret, body, &v.to_owned()) {
                false => Err(SecretError::Invalid),
                true => Ok(()),
            },
        },
    }
}

/// Compare a valid signature for a payload against that offered alongside it
/// in a request.
fn is_valid_signature(secret: &HerokuSecret, payload: &Bytes, sig: &String) -> bool {
    gen_signature(secret, payload).as_ref() == Some(sig)
}

/// Generate a valid signature with our secret for a payload.
fn gen_signature(secret: &HerokuSecret, payload: &Bytes) -> Option<String> {
    type HmacSha256 = Hmac<Sha256>;

    HmacSha256::new_from_slice(secret.0.as_bytes())
        .map(|mut mac| {
            mac.update(payload);
            b64.encode(mac.finalize().into_bytes())
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_signature() {
        let secret = HerokuSecret(String::from("foobar"));
        let payload = String::from("a wild payload appeared");
        let valid_sig = String::from("luDEVkRg2AxxcflGmamyN5mOPleyccUZdkg+C0MoRBY=");

        assert!(is_valid_signature(
            &secret,
            &Bytes::from(payload.clone()),
            &valid_sig
        ));
        assert!(!is_valid_signature(
            &secret,
            &Bytes::from(payload),
            &String::from("invalid signature")
        ));
    }

    /// As a sanity check you can get the same output in JavaScript:
    ///
    /// ```js
    /// const genSig = (secret, payload) =>
    ///   crypto
    ///     .createHmac('sha256', secret)
    ///     .update(Buffer.from(payload))
    ///     .digest('base64')
    /// ```
    #[test]
    fn test_gen_signature() {
        let secret = HerokuSecret(String::from("foobar"));
        let payload = String::from("a wild payload appeared");
        let expected = String::from("luDEVkRg2AxxcflGmamyN5mOPleyccUZdkg+C0MoRBY=");

        assert_eq!(
            gen_signature(&secret, &Bytes::from(payload)),
            Some(expected)
        );
    }
}
