use jwt_rustcrypto::{decode, Error, VerifyingKey, ValidationOptions, Algorithm};

byond_fn!(fn decode_jwt_token(token, key) {
    match decode_token(token, key) {
        Ok(r) => return Some(r),
        Err(e) => return Some(e.to_string())
    }
});

pub fn decode_token(token: &str, key: &str) -> Result<String, Error> {
    let verification_key = VerifyingKey::from_secret(key.as_bytes());
    let validation_options = ValidationOptions::default().with_algorithm(Algorithm::RS256).without_expiry();
    let decoded = decode(&token, &verification_key, &validation_options).expect("Failed to decode or validate JWT with RSA key");
    Ok(serde_json::to_string(&decoded.payload)?)
}
