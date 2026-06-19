use solana_address::Address;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::str::FromStr;

pub struct RelayerConfig {
    pub relayer_fee_payer_keypair: Keypair,
    pub relayer_token_account: Address,
    pub min_service_fee_usdc: u64,
}

#[derive(Debug)]
pub enum ValidationError {
    InvalidTransaction,
    MissingFeeInstruction,
    FeeTooLow,
    SimulationFailed,
}

pub fn validate_and_cosign_transaction(
    tx_bytes: &[u8],
    config: &RelayerConfig,
) -> Result<VersionedTransaction, ValidationError> {
    let mut versioned_tx: VersionedTransaction = bincode::deserialize(tx_bytes)
        .map_err(|_| ValidationError::InvalidTransaction)?;

    let message = &versioned_tx.message;
    let mut fee_allocated = 0u64;
    let mut pays_relayer = false;

    let token_program =
        Address::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|_| ValidationError::InvalidTransaction)?;

    for instruction in message.instructions() {
        let program_id = message.static_account_keys()[instruction.program_id_index as usize];

        if program_id == token_program {
            let accounts = &instruction.accounts;

            if accounts.len() >= 2 {
                let dest_account_index = accounts[1] as usize;
                let destination = message.static_account_keys()[dest_account_index];

                if destination == config.relayer_token_account {
                    pays_relayer = true;

                    if instruction.data.len() >= 9 && instruction.data[0] == 3 {
                        let mut amount_bytes = [0u8; 8];
                        amount_bytes.copy_from_slice(&instruction.data[1..9]);
                        fee_allocated = u64::from_le_bytes(amount_bytes);
                    }
                }
            }
        }
    }

    if !pays_relayer {
        return Err(ValidationError::MissingFeeInstruction);
    }

    if fee_allocated < config.min_service_fee_usdc {
        return Err(ValidationError::FeeTooLow);
    }

    let signature = config
        .relayer_fee_payer_keypair
        .sign_message(&message.serialize());
    versioned_tx.signatures[0] = signature;

    Ok(versioned_tx)
}
