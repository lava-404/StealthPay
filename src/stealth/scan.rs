use curve25519_dalek::{
    constants::ED25519_BASEPOINT_POINT as G,
    edwards::CompressedEdwardsY,
    scalar::Scalar,
    EdwardsPoint,
};
use sha2::{Digest, Sha256};
use solana_address::Address;
use solana_transaction::versioned::VersionedTransaction;
use std::str::FromStr;

const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

pub fn detect_incoming_stealth_payment(
    tx: &VersionedTransaction,
    private_view_key: &Scalar,
    public_spend_key: &EdwardsPoint,
) -> Option<Address> {
    let message = &tx.message;
    let account_keys = message.static_account_keys();

    let mut ephemeral_pubkey_bytes = None;
    let mut target_stealth_address = None;

    for instruction in message.instructions() {
        let program_id = account_keys[instruction.program_id_index as usize];

        if program_id == Address::from_str(MEMO_PROGRAM_ID).ok()? {
            if let Ok(memo_str) = std::str::from_utf8(&instruction.data) {
                let clean_memo = memo_str.replace("ephemeral:", "");
                if let Ok(decoded_bytes) = bs58::decode(clean_memo.trim()).into_vec() {
                    if decoded_bytes.len() == 32 {
                        ephemeral_pubkey_bytes = Some(decoded_bytes);
                    }
                }
            }
        }
    }

    let raw_ephemeral = ephemeral_pubkey_bytes?;
    let ephemeral_point = CompressedEdwardsY::from_slice(&raw_ephemeral)
        .ok()?
        .decompress()?;

    let shared_secret = private_view_key * ephemeral_point;

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.compress().as_bytes());
    let hashed_secret: [u8; 32] = hasher.finalize().into();
    let e = Scalar::from_bytes_mod_order(hashed_secret);

    let expected_stealth_point = public_spend_key + &(e * G);
    let expected_stealth_bytes = expected_stealth_point.compress().to_bytes();

    for account in account_keys {
        if account.as_array() == &expected_stealth_bytes {
            target_stealth_address = Some(*account);
            break;
        }
    }

    target_stealth_address
}
