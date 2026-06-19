use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT as G, scalar::Scalar, EdwardsPoint};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use solana_address::Address;

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

pub fn generate_recipient_keys() -> RecipientKeys {
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

pub fn derive_stealth_address(meta: &MetaAddress) -> (Address, EdwardsPoint) {
    let mut rng = OsRng;
    let mut ephemeral_bytes = [0u8; 32];
    rng.fill_bytes(&mut ephemeral_bytes);

    let sender_secret = Scalar::from_bytes_mod_order(ephemeral_bytes);
    let ephemeral_public = &sender_secret * G;

    let shared_secret = &sender_secret * &meta.view_public;

    let mut hasher = Sha256::new();
    hasher.update(shared_secret.compress().as_bytes());
    let hashed_secret: [u8; 32] = hasher.finalize().into();

    let scalar = Scalar::from_bytes_mod_order(hashed_secret);
    let stealth_point: EdwardsPoint = &scalar * G + &meta.spend_public;

    let stealth_bytes: [u8; 32] = stealth_point.compress().to_bytes();
    let stealth_address = Address::from(stealth_bytes);

    (stealth_address, ephemeral_public)
}
