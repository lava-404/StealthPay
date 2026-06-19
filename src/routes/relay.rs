use axum::{extract::State, Json, response::IntoResponse};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::json;
use solana_client::rpc_config::RpcSendTransactionConfig;
use std::sync::Arc;

use crate::relayer::validate::validate_and_cosign_transaction;
use crate::AppState;

#[derive(Deserialize)]
pub struct RelayRequest {
    pub transaction_base64: String,
}

// POST /relay
pub async fn relay_transaction(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RelayRequest>,
) -> impl IntoResponse {
    let tx_bytes = match STANDARD.decode(&payload.transaction_base64) {
        Ok(bytes) => bytes,
        Err(_) => return Json(json!({"error": "Invalid base64 payload"})),
    };

    let signed_tx = match validate_and_cosign_transaction(&tx_bytes, &state.relayer_config) {
        Ok(tx) => tx,
        Err(e) => {
            return Json(json!({"error": format!("Validation failed: {:?}", e)}));
        }
    };

    let config = RpcSendTransactionConfig {
        skip_preflight: false,
        ..Default::default()
    };

    match state
        .rpc_client
        .send_transaction_with_config(&signed_tx, config)
    {
        Ok(signature) => Json(json!({
            "status": "success",
            "signature": signature.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}
