use curve25519_dalek::{scalar::Scalar, EdwardsPoint};
use sha2::{Digest, Sha256};
use solana_keypair::Keypair;

pub fn recover_stealth_private_key(
    ephemeral_pubkey: &EdwardsPoint,
    private_view_key: &Scalar,
    private_spend_key: &Scalar,
) -> Keypair {
    let shared_secret = private_view_key * ephemeral_pubkey;

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.compress().as_bytes());
    let hashed_secret: [u8; 32] = hasher.finalize().into();

    let e = Scalar::from_bytes_mod_order(hashed_secret);
    let stealth_private_scalar = private_spend_key + e;

    let seed = *stealth_private_scalar.as_bytes();
    Keypair::new_from_array(seed)
}
