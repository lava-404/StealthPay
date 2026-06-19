use axum::{extract::State, Json, response::IntoResponse};
use curve25519_dalek::edwards::CompressedEdwardsY;
use serde::Deserialize;
use serde_json::json;
use solana_address::Address;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;

use crate::stealth::derive::{derive_stealth_address, MetaAddress};
use crate::AppState;

#[derive(Deserialize)]
pub struct SendRequest {
    pub view_public_bs58: String,
    pub spend_public_bs58: String,
    pub amount_sol: f64,
    pub sender_private_key_bs58: String,
}

fn decompress_point(bytes: &[u8]) -> Option<curve25519_dalek::EdwardsPoint> {
    let arr: [u8; 32] = bytes.try_into().ok()?;
    CompressedEdwardsY::from_slice(&arr)
        .ok()?
        .decompress()
}

// POST /send
pub async fn send_stealth_payment(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendRequest>,
) -> impl IntoResponse {
    let view_bytes = match bs58::decode(&payload.view_public_bs58).into_vec() {
        Ok(bytes) => bytes,
        Err(_) => return Json(json!({"error": "Invalid view_public_bs58"})),
    };
    let spend_bytes = match bs58::decode(&payload.spend_public_bs58).into_vec() {
        Ok(bytes) => bytes,
        Err(_) => return Json(json!({"error": "Invalid spend_public_bs58"})),
    };
    let sender_keypair = match Keypair::try_from_base58_string(&payload.sender_private_key_bs58) {
        Ok(keypair) => keypair,
        Err(_) => return Json(json!({"error": "Invalid sender_private_key_bs58"})),
    };

    let view_public = match decompress_point(&view_bytes) {
        Some(point) => point,
        None => return Json(json!({"error": "Invalid view public key"})),
    };
    let spend_public = match decompress_point(&spend_bytes) {
        Some(point) => point,
        None => return Json(json!({"error": "Invalid spend public key"})),
    };

    let meta = MetaAddress {
        view_public,
        spend_public,
    };

    let (stealth_address, ephemeral_public) = derive_stealth_address(&meta);
    let amount_lamports = (payload.amount_sol * 1_000_000_000.0) as u64;

    let recent_blockhash = match state.rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(e) => {
            return Json(json!({
                "status": "error",
                "message": format!("Failed to fetch blockhash: {e}")
            }));
        }
    };

    let transfer_ix = system_instruction::transfer(
        &sender_keypair.pubkey(),
        &stealth_address,
        amount_lamports,
    );

    let memo_program =
        Address::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr").unwrap();
    let memo = format!(
        "ephemeral:{}",
        bs58::encode(ephemeral_public.compress().to_bytes()).into_string()
    );

    let memo_ix = Instruction {
        program_id: memo_program,
        accounts: vec![],
        data: memo.into_bytes(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix, memo_ix],
        Some(&sender_keypair.pubkey()),
        &[sender_keypair],
        recent_blockhash,
    );

    match state.rpc_client.send_and_confirm_transaction(&tx) {
        Ok(signature) => Json(json!({
            "status": "success",
            "signature": signature.to_string(),
            "stealth_address": stealth_address.to_string()
        })),
        Err(e) => Json(json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}
