use anyhow::{anyhow, Context};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use log::warn;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// Default claims type when no specific structure is needed.
/// Allows claims to be any valid JSON object.
pub type AnyClaims = serde_json::Value;

/// Validates JWTs signed with the ES256 algorithm.
///
/// Public keys are loaded from a YAML file where each entry maps a key
/// identifier (`kid`) to its PEM-encoded public key.
pub struct JwtValidator {
    keys: HashMap<String, DecodingKey>,
    validation: Validation,
}

impl JwtValidator {
    /// Loads public keys from a YAML file.
    ///
    /// The file must follow this format:
    /// ```yaml
    /// key-id-1: |
    ///   -----BEGIN PUBLIC KEY-----
    ///   ...
    ///   -----END PUBLIC KEY-----
    /// ```
    ///
    /// Returns an error if the file is not found, malformed, or contains an
    /// invalid PEM key.
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Unable to read JWT keys file {}", path))?;
        let raw_keys: HashMap<String, String> =
            serde_yaml::from_str(&content).context("Invalid keys.yaml format")?;

        if raw_keys.is_empty() {
            warn!("No key found in keys file");
        }

        let mut keys = HashMap::with_capacity(raw_keys.len());
        for (id, pem) in raw_keys {
            let decoding_key = DecodingKey::from_ec_pem(pem.trim().as_bytes())
                .with_context(|| format!("Invalid PEM for key id {}", id))?;
            keys.insert(id, decoding_key);
        }

        let validation = Validation::new(Algorithm::ES256);

        Ok(Self { keys, validation })
    }

    /// Returns the number of loaded public keys.
    pub fn keys_count(&self) -> usize {
        self.keys.len()
    }

    /// Validates a JWT passed in an `Authorization: Bearer <token>` header and
    /// deserializes its claims into `C`.
    ///
    /// The following checks are performed:
    /// - presence of the `Bearer` prefix
    /// - ES256 algorithm
    /// - signature against the known keys (targeting `kid` when present)
    /// - token expiry
    ///
    /// Returns the deserialized claims on success, or a descriptive error otherwise.
    pub fn validate_bearer_token<C: DeserializeOwned>(
        &self,
        authorization: &str,
    ) -> anyhow::Result<C> {
        let token = authorization
            .strip_prefix("Bearer ")
            .ok_or_else(|| anyhow!("Missing Bearer prefix"))?
            .trim();

        if token.is_empty() {
            return Err(anyhow!("Bearer token is empty"));
        }

        let header = decode_header(token).context("Invalid JWT header")?;
        if header.alg != Algorithm::ES256 {
            return Err(anyhow!("Unsupported algorithm {:?}", header.alg));
        }

        if let Some(kid) = header.kid {
            let Some(key) = self.keys.get(&kid) else {
                return Err(anyhow!("Unknown key id {}", kid));
            };

            let claims =
                decode::<C>(token, key, &self.validation).context("Invalid JWT signature")?;
            return Ok(claims.claims);
        }

        if let Some(claims) = self
            .keys
            .values()
            .find_map(|key| decode::<C>(token, key, &self.validation).ok())
        {
            return Ok(claims.claims);
        }

        Err(anyhow!("Invalid JWT signature"))
    }
}

/// JWT integration for the [warp](https://docs.rs/warp) framework.
///
/// Requires the `warp` feature.
#[cfg(feature = "warp")]
pub mod warp {
    use super::JwtValidator;
    use log::warn;
    use serde_json::Value;
    use std::sync::Arc;
    use warp::{Filter, Rejection};

    /// Warp rejection error emitted when a request is not authenticated.
    #[derive(Debug)]
    pub struct Unauthorized;

    impl warp::reject::Reject for Unauthorized {}

    /// Authentication mode for the [`with_auth`] filter.
    #[derive(Clone)]
    pub enum AuthMode {
        /// Validates the JWT token using the provided [`JwtValidator`].
        Validate(Arc<JwtValidator>),
        /// Disables JWT verification (intended for non-production environments).
        SkipAuthentication,
    }

    /// Builds a Warp filter that enforces the `Authorization: Bearer` header.
    ///
    /// - In [`AuthMode::Validate`] mode, the JWT token must be present and valid.
    /// - In [`AuthMode::SkipAuthentication`] mode, all requests are accepted
    ///   without verification (should only be used in non-production environments).
    ///
    /// On failure, the request is rejected with [`Unauthorized`].
    pub fn with_auth(
        auth_mode: AuthMode,
    ) -> impl Filter<Extract = ((),), Error = Rejection> + Clone {
        warp::header::optional::<String>("authorization").and_then(
            move |authorization: Option<String>| {
                let auth_mode = auth_mode.clone();
                async move {
                    if let AuthMode::SkipAuthentication = auth_mode {
                        return Ok(());
                    }

                    match authorization {
                        Some(header) => {
                            match auth_mode {
                                AuthMode::Validate(validator) => {
                                    validator.validate_bearer_token::<Value>(&header).map_err(
                                        |e| {
                                            warn!("Unauthorized request: {}", e);
                                            warp::reject::custom(Unauthorized)
                                        },
                                    )?;
                                }
                                AuthMode::SkipAuthentication => unreachable!(),
                            }
                            Ok(())
                        }
                        None => Err(warp::reject::custom(Unauthorized)),
                    }
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_EC_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
        MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgmNfAAEnuuRnUc3yN\n\
        g85n2iFM6egVGfaGJAa5TtL/edChRANCAATVu/tAGCPjGFNdj4wNxneTtthmOwVw\n\
        1hnxJt/3HkxtjKEONNuZsuGSiF+mouqJQrOYl0mst7pQeM0jOtiLIxtz\n\
        -----END PRIVATE KEY-----\n";

    const TEST_EC_PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
        MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE1bv7QBgj4xhTXY+MDcZ3k7bYZjsF\n\
        cNYZ8Sbf9x5MbYyhDjTbmbLhkohfpqLqiUKzmJdJrLe6UHjNIzrYiyMbcw==\n\
        -----END PUBLIC KEY-----\n";

    fn create_keys_yaml(entries: &[(&str, &str)]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        for (id, pub_key_pem) in entries {
            writeln!(file, "{}: {:?}", id, pub_key_pem).unwrap();
        }
        file.flush().unwrap();
        file
    }

    fn make_token(kid: Option<&str>) -> String {
        let key = EncodingKey::from_ec_pem(TEST_EC_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        let mut header = Header::new(Algorithm::ES256);
        header.kid = kid.map(|s| s.to_string());
        let claims = serde_json::json!({"sub": "test", "exp": 9999999999u64});
        encode(&header, &claims, &key).unwrap()
    }

    // --- load_from_file ---

    #[test]
    fn load_from_file_with_valid_key() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(validator.keys_count(), 1);
    }

    #[test]
    fn load_from_file_with_multiple_keys() {
        let file = create_keys_yaml(&[
            ("key1", TEST_EC_PUBLIC_KEY_PEM),
            ("key2", TEST_EC_PUBLIC_KEY_PEM),
        ]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(validator.keys_count(), 2);
    }

    #[test]
    fn load_from_file_nonexistent_path() {
        let result = JwtValidator::load_from_file("/tmp/nonexistent_keys_xyz.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file_invalid_yaml() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "[not a valid yaml map").unwrap();
        file.flush().unwrap();
        let result = JwtValidator::load_from_file(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file_invalid_pem() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "bad-key: \"not-a-valid-pem\"").unwrap();
        file.flush().unwrap();
        let result = JwtValidator::load_from_file(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file_empty_keys() {
        let file = create_keys_yaml(&[]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(validator.keys_count(), 0);
    }

    // --- validate_bearer_token ---

    #[test]
    fn validate_valid_token_with_kid() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let token = make_token(Some("test-key"));
        let auth = format!("Bearer {}", token);
        assert!(validator.validate_bearer_token::<AnyClaims>(&auth).is_ok());
    }

    #[test]
    fn validate_valid_token_without_kid() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let token = make_token(None);
        let auth = format!("Bearer {}", token);
        assert!(validator.validate_bearer_token::<AnyClaims>(&auth).is_ok());
    }

    #[test]
    fn validate_token_with_unknown_kid() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let token = make_token(Some("unknown-key"));
        let auth = format!("Bearer {}", token);
        let result = validator.validate_bearer_token::<AnyClaims>(&auth);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown key id"));
    }

    #[test]
    fn validate_missing_bearer_prefix() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let token = make_token(Some("test-key"));
        let result = validator.validate_bearer_token::<AnyClaims>(&token);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing Bearer"));
    }

    #[test]
    fn validate_empty_bearer_token() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let result = validator.validate_bearer_token::<AnyClaims>("Bearer ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn validate_garbage_token() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();
        let result = validator.validate_bearer_token::<AnyClaims>("Bearer not.a.valid.jwt");
        assert!(result.is_err());
    }

    #[test]
    fn validate_token_wrong_key() {
        // Create a validator with no keys to test rejection
        let mut file = NamedTempFile::new().unwrap();
        // Write an empty keys yaml (no keys)
        file.flush().unwrap();
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap());
        // With no keys, any token without kid should fail
        if let Ok(v) = validator {
            let token = make_token(None);
            let auth = format!("Bearer {}", token);
            assert!(v.validate_bearer_token::<AnyClaims>(&auth).is_err());
        }
    }

    #[test]
    fn validate_expired_token() {
        let file = create_keys_yaml(&[("test-key", TEST_EC_PUBLIC_KEY_PEM)]);
        let validator = JwtValidator::load_from_file(file.path().to_str().unwrap()).unwrap();

        let key = EncodingKey::from_ec_pem(TEST_EC_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some("test-key".to_string());
        // exp in the past
        let claims = serde_json::json!({"sub": "test", "exp": 1000000000u64});
        let token = encode(&header, &claims, &key).unwrap();
        let auth = format!("Bearer {}", token);

        let result = validator.validate_bearer_token::<AnyClaims>(&auth);
        assert!(result.is_err());
    }
}
