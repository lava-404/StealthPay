use axum::{
    extract::{Query, State},
    Json,
    response::IntoResponse,
};
use curve25519_dalek::{edwards::CompressedEdwardsY, scalar::Scalar};
use serde::Deserialize;
use serde_json::json;
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client_api::config::RpcTransactionConfig;
use solana_signature::Signature;
use solana_transaction_status_client_types::UiTransactionEncoding;
use std::str::FromStr;
use std::sync::Arc;

use crate::stealth::scan::detect_incoming_stealth_payment;
use crate::AppState;

#[derive(Deserialize)]
pub struct PaymentsQuery {
    pub view_private_bs58: String,
    pub spend_public_bs58: String,
    #[serde(default)]
    pub signatures: String,
}

fn decompress_point(bytes: &[u8]) -> Option<curve25519_dalek::EdwardsPoint> {
    let arr: [u8; 32] = bytes.try_into().ok()?;
    CompressedEdwardsY::from_slice(&arr)
        .ok()?
        .decompress()
}

// GET /payments?signatures=sig1,sig2
pub async fn get_payments(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaymentsQuery>,
) -> impl IntoResponse {
    let view_bytes = match bs58::decode(&query.view_private_bs58).into_vec() {
        Ok(bytes) if bytes.len() == 32 => bytes,
        _ => return Json(json!({"error": "Invalid view_private_bs58"})),
    };
    let spend_bytes = match bs58::decode(&query.spend_public_bs58).into_vec() {
        Ok(bytes) if bytes.len() == 32 => bytes,
        _ => return Json(json!({"error": "Invalid spend_public_bs58"})),
    };

    let view_private = Scalar::from_bytes_mod_order(view_bytes.try_into().unwrap());
    let spend_public = match decompress_point(&spend_bytes) {
        Some(point) => point,
        None => return Json(json!({"error": "Invalid spend public key"})),
    };

    if query.signatures.trim().is_empty() {
        return Json(json!({
            "status": "ok",
            "payments": [],
            "message": "Provide comma-separated transaction signatures to scan"
        }));
    }

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Base64),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let mut payments = Vec::new();

    for signature_str in query.signatures.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let signature = match Signature::from_str(signature_str) {
            Ok(sig) => sig,
            Err(_) => {
                payments.push(json!({
                    "signature": signature_str,
                    "error": "Invalid signature format"
                }));
                continue;
            }
        };

        let encoded_tx = match state
            .rpc_client
            .get_transaction_with_config(&signature, config.clone())
        {
            Ok(tx) => tx,
            Err(e) => {
                payments.push(json!({
                    "signature": signature_str,
                    "error": e.to_string()
                }));
                continue;
            }
        };

        let Some(versioned_tx) = encoded_tx.transaction.transaction.decode() else {
            payments.push(json!({
                "signature": signature_str,
                "error": "Failed to decode transaction"
            }));
            continue;
        };

        if let Some(stealth_address) =
            detect_incoming_stealth_payment(&versioned_tx, &view_private, &spend_public)
        {
            payments.push(json!({
                "signature": signature_str,
                "stealth_address": stealth_address.to_string(),
                "detected": true
            }));
        } else {
            payments.push(json!({
                "signature": signature_str,
                "detected": false
            }));
        }
    }

    Json(json!({
        "status": "ok",
        "payments": payments
    }))
}
