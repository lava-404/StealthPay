use axum::{Json, response::IntoResponse};
use serde_json::json;
use crate::stealth::derive::generate_recipient_keys;

// POST /meta-address/create
pub async fn create_meta_address() -> impl IntoResponse {
    let keys = generate_recipient_keys();

    // In a real app, you wouldn't return the private keys! 
    // This is for testing the cryptography.
    let response = json!({
        "view_public": bs58::encode(keys.view_public.compress().as_bytes()).into_string(),
        "spend_public": bs58::encode(keys.spend_public.compress().as_bytes()).into_string(),
        "view_private": bs58::encode(keys.view_private.as_bytes()).into_string(),
        "spend_private": bs58::encode(keys.spend_private.as_bytes()).into_string(),
    });

    Json(response)
}