use curve25519_dalek::{EdwardsPoint, scalar::Scalar};
use curve25519_dalek::constants::ED25519_BASEPOINT_POINT as G;
use rand::rngs::OsRng;
use rand::RngCore;
use blake3::hash;
use solana_address::Address;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_system_interface::instruction as system_instruction;
use solana_transaction::Transaction;
use solana_instruction::{
    AccountMeta,
    Instruction,
};
use sha2::{Digest, Sha256};
use solana_sdk::{
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::str::FromStr;
const WS_URL: &str= "wss://devnet.helius-rpc.com/?api-key=ffe3568c-a4ff-4b2f-a2f5-53d891278489";
pub struct MetaAddress {
    pub view_public: EdwardsPoint,
    pub spend_public: EdwardsPoint,
}

pub struct RelayerConfig {
    pub relayer_fee_payer_keypair: Keypair,
    pub relayer_token_account: Pubkey, // Where you want to receive your USDC fee
    pub min_service_fee_usdc: u64,      // E.g., 1_000_000 (which is 1 USDC, assuming 6 decimals)
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidTransaction,
    MissingFeeInstruction,
    FeeTooLow,
    SimulationFailed,
}

pub struct RecipientKeys {
    pub view_private: Scalar,
    pub spend_private: Scalar,
    pub view_public: EdwardsPoint,
    pub spend_public: EdwardsPoint,
}

// Helper to generate valid keys
fn generate_recipient_keys() -> RecipientKeys {
    let mut rng = OsRng;
    let mut view_bytes = [0u8; 32];
    let mut spend_bytes = [0u8; 32];
    rng.fill_bytes(&mut view_bytes);
    rng.fill_bytes(&mut spend_bytes);

    let view_private = Scalar::from_bytes_mod_order(view_bytes);
    let spend_private = Scalar::from_bytes_mod_order(spend_bytes);

    RecipientKeys {
        view_private,
        spend_private,
        view_public: &view_private * G,
        spend_public: &spend_private * G,
    }
}

fn stealth_sender(
    rpc_client: &RpcClient,
    meta_address: &RecipientKeys,
    sender_keypair: &Keypair,
    amount_sol: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Convert Solana Keypair to curve25519_dalek Scalar
    // The secret key is the first 32 bytes of the keypair
    let mut rng = OsRng;

    let mut ephemeral_bytes = [0u8; 32];
    rng.fill_bytes(&mut ephemeral_bytes);

    let sender_secret =
    Scalar::from_bytes_mod_order(ephemeral_bytes);
    let ephemeral_public = &sender_secret * G;
    let ephemeral_public_bytes: [u8; 32] = ephemeral_public.compress().to_bytes();
    let ephemeral_public_address = Address::from(ephemeral_public_bytes);

    // 2. Perform Stealth Derivation
    let shared_secret = &sender_secret * &meta_address.view_public;
    let hashed_secret = hash(&shared_secret.compress().to_bytes());
    let scalar = Scalar::from_bytes_mod_order(*hashed_secret.as_bytes());

    let stealth_point: EdwardsPoint = &scalar * G + &meta_address.spend_public;

    // 3. Convert EdwardsPoint to Solana Address
    let stealth_bytes: [u8; 32] = stealth_point.compress().to_bytes();
    let stealth_address = Address::from(stealth_bytes);
    

    // 4. Convert SOL to Lamports
    let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

    // 5. Construct and Send Transaction
    let recent_blockhash = rpc_client.get_latest_blockhash()?;

    let transfer_ix = system_instruction::transfer(
        &sender_keypair.pubkey(),
        &stealth_address,
        amount_lamports,
    );
    let memo_program =
    Address::from_str(
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"
    )?;
    let memo = format!(
        "ephemeral:{}",
        bs58::encode(
            ephemeral_public_address
        ).into_string()
    );
    
    let memo_ix = Instruction {
        program_id: memo_program,
        accounts: vec![],
        data: memo.as_bytes().to_vec(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix, memo_ix],
        Some(&sender_keypair.pubkey()),
        &[sender_keypair],
        recent_blockhash,
    );
    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    println!("Transaction confirmed: {}", signature);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let recipient_keys = generate_recipient_keys();
    let sender_keypair = Keypair::new();
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());

    stealth_sender(&rpc_client, &recipient_keys, &sender_keypair, 0.001)?;

    Ok(())
}



//monitor the newly confirmed transactions via websocket
//find the stealth address 
//reciever has to take money from the stealth address


/// Phase 2 & 3: Introspects the transaction payload before the Relayer signs it
pub fn validate_and_cosign_transaction(
    tx_bytes: &[u8],
    config: &RelayerConfig,
) -> Result<VersionedTransaction, ValidationError> {
    // 1. Deserialize the raw transaction bytes sent by the user
    let mut versioned_tx: VersionedTransaction = bincode::deserialize(tx_bytes)
        .map_err(|_| ValidationError::InvalidTransaction)?;

    // 2. Parse the transaction message and look at the instructions
    let message = &versioned_tx.message;
    let mut fee_allocated = 0u64;
    let mut pays_relayer = false;

    // We crawl through the instructions to find the transfer paying the relayer
    for instruction in message.instructions() {
        let program_id = message.static_account_keys()[instruction.program_id_index as usize];
        
        // Check if the instruction is pointing to the SPL Token Program (or native System Program)
        // For simplicity, let's assume an SPL Token transfer (USDC)
        if program_id == Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap() {
            let accounts = &instruction.accounts;
            
            // Check if the destination of this token transfer is your Relayer's account
            if accounts.len() >= 2 {
                let dest_account_index = accounts[1] as usize;
                let destination_pubkey = message.static_account_keys()[dest_account_index];

                if destination_pubkey == config.relayer_token_account {
                    pays_relayer = true;
                    
                    // Decode the helper fee amount from the instruction data payload
                    // Standard SPL Token transfer instruction has the amount in bytes 1..9
                    if instruction.data.len() >= 9 && instruction.data[0] == 3 { // 3 = Transfer instruction
                        let mut amount_bytes = [0u8; 8];
                        amount_bytes.copy_from_slice(&instruction.data[1..9]);
                        fee_allocated = u64::from_le_bytes(amount_bytes);
                    }
                }
            }
        }
    }

    // 3. Reject if the user is trying to get free gas without paying you
    if !pays_relayer {
        return Err(ValidationError::MissingFeeInstruction);
    }

    if fee_allocated < config.min_service_fee_usdc {
        return Err(ValidationError::FeeTooLow);
    }

    // 4. Safely sign the transaction as the Fee Payer!
    // Since we are the Fee Payer, we add our signature to the transaction
    versioned_tx.try_sign(&[&config.relayer_fee_payer_keypair])
        .map_err(|_| ValidationError::SimulationFailed)?;

    Ok(versioned_tx)
}

//aync 

pub fn recover_stealth_private_key(
    ephemeral_pubkey: &EdwardsPoint,
    private_view_key: &Scalar,
    private_spend_key: &Scalar,
) -> Keypair {
    // 1. Reconstruct the Shared Secret: S = private_view_key * Ephemeral_Pubkey
    let shared_secret = private_view_key * ephemeral_pubkey;

    // 2. Hash the Shared Secret to regenerate the blinding factor e
    let mut hasher = Sha256::new();
    hasher.update(shared_secret.compress().as_bytes());
    let hashed_secret = hasher.finalize();
    let e = Scalar::from_bytes_mod_order_wide(hashed_secret.as_slice().try_into().unwrap());

    // 3. Recover Stealth Private Key: p = private_spend_key + e
    let stealth_private_scalar = private_spend_key + e;

    // 4. Convert the scalar back into a Solana Keypair
    let mut seed = [0u8; 32];
    seed.copy_from_slice(stealth_private_scalar.as_bytes());
    
    Keypair::from_seed(&seed).expect("Failed to derive valid Solana keypair from scalar seed")
}

const MEMO_PROGRAM_ID: &str = "Memorigi41niaw5thnZ9kvyjjm7vYg79kiA4ES1Ed1";

/// Scans a single transaction to see if it contains a stealth payment meant for this recipient.
/// Returns `Some(Pubkey)` (the stealth address) if a match is found, otherwise `None`.
pub fn detect_incoming_stealth_payment(
    tx: &VersionedTransaction,
    private_view_key: &Scalar,
    public_spend_key: &EdwardsPoint,
) -> Option<Pubkey> {
    let message = &tx.message;
    let account_keys = message.static_account_keys();
    
    let mut ephemeral_pubkey_bytes = None;
    let mut target_stealth_address = None;

    // 1. Parse instructions to extract the Ephemeral Key from the Memo
    for instruction in message.instructions() {
        let program_id = account_keys[instruction.program_id_index as usize];
        
        if program_id == Pubkey::from_str(MEMO_PROGRAM_ID).unwrap() {
            // The memo data contains the base58-encoded Ephemeral Public Key string
            if let Ok(memo_str) = std::str::from_utf8(&instruction.data) {
                if let Ok(decoded_bytes) = bs58::decode(memo_str.trim()).into_vec() {
                    if decoded_bytes.len() == 32 {
                        ephemeral_pubkey_bytes = Some(decoded_bytes);
                    }
                }
            }
        }
    }

    // If no valid 32-byte ephemeral key was passed in the memo, this isn't a stealth tx
    let raw_ephemeral = ephemeral_pubkey_bytes?;
    let ephemeral_point = CompressedEdwardsY::from_slice(&raw_ephemeral).decompress()?;

    // 2. Perform ECDH Math to derive the expected Stealth Address
    // Shared Secret: S = private_view_key * Ephemeral_Point
    let shared_secret = private_view_key * ephemeral_point;

    // Blinding Factor: e = SHA256(S)
    let mut hasher = Sha256::new();
    hasher.update(shared_secret.compress().as_bytes());
    let hashed_secret = hasher.finalize();
    let e = Scalar::from_bytes_mod_order_wide(hashed_secret.as_slice().try_into().unwrap());

    // Expected Stealth Point: P = public_spend_key + (e * G)
    let e_g = &e * &ED25519_BASEPOINT_TABLE;
    let expected_stealth_point = public_spend_key + e_g;
    let expected_stealth_bytes = expected_stealth_point.compress().to_bytes();

    // 3. Check if this expected address actually received tokens in the transaction
    for account in account_keys {
        if account.to_bytes() == expected_stealth_bytes {
            target_stealth_address = Some(*account);
            break;
        }
    }

    target_stealth_address
}