//! Helpers around Heroku's use of a shared secret to authenticate webhook
//! requests.
//!
//! Requests are validated with a secret that's given to Heroku when
//! initialising the webhook. The secret is used to sign the request body, the
//! result of which is included in a header. We'll compare our own signature
//! against it to know if the request really came from Heroku.
//!
//! <https://devcenter.heroku.com/articles/app-webhooks#using-the-shared-secret>

use base64::{engine::general_purpose::STANDARD as b64, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// A newtype wrapper around the Heroku secret.
pub struct HerokuSecret(pub String);

/// Compare a valid signature for a payload against that offered alongside it
/// in a request. Requests which fail this predicate, or which don't have a
/// signature at all, should be considered unauthenticated.
pub fn is_valid_signature(secret: &HerokuSecret, payload: &String, sig: &String) -> bool {
    gen_signature(secret, payload).as_ref() == Some(sig)
}

/// Generate a valid signature with our secret for a payload.
fn gen_signature(secret: &HerokuSecret, payload: &String) -> Option<String> {
    type HmacSha256 = Hmac<Sha256>;

    HmacSha256::new_from_slice(secret.0.as_bytes())
        .map(|mut mac| {
            mac.update(payload.as_bytes());
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

        assert!(is_valid_signature(&secret, &payload, &valid_sig));
        assert!(!is_valid_signature(
            &secret,
            &payload,
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

        assert_eq!(gen_signature(&secret, &payload), Some(expected));
    }
}
