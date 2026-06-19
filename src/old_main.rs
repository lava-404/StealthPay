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
use std::str::FromStr;
const WS_URL: &str= "wss://devnet.helius-rpc.com/?api-key=ffe3568c-a4ff-4b2f-a2f5-53d891278489";
pub struct MetaAddress {
    pub view_public: EdwardsPoint,
    pub spend_public: EdwardsPoint,
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
    let secret_bytes = &sender_keypair.to_bytes()[0..32];
    let sender_secret = Scalar::from_bytes_mod_order(secret_bytes.try_into().unwrap());

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
