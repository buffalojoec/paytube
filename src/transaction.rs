use {
    solana_sdk::{
        instruction::Instruction as SolanaInstruction,
        pubkey::Pubkey,
        system_instruction,
        transaction::{
            SanitizedTransaction as SolanaSanitizedTransaction, Transaction as SolanaTransaction,
        },
    },
    std::collections::HashSet,
};

/// A simple PayTube transaction.
///
/// Transfers SPL tokens or SOL from one account to another.
pub struct PayTubeTransaction {
    /// The SPL Token mint to transfer. A `None` value represents native SOL.
    pub mint: Option<Pubkey>,
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

impl PayTubeTransaction {
    /// Create a batch of Solana transactions, for the Solana SVM's transaction
    /// processor, from a batch of PayTube instructions.
    pub fn create_svm_transactions(
        paytube_instructions: &[Self],
    ) -> Vec<SolanaSanitizedTransaction> {
        let reserved_account_keys = HashSet::new();
        paytube_instructions
            .iter()
            .map(|instruction| {
                SolanaSanitizedTransaction::try_from_legacy_transaction(
                    SolanaTransaction::new_with_payer(
                        &[SolanaInstruction::from(instruction)],
                        Some(&instruction.from),
                    ),
                    &reserved_account_keys,
                )
                .unwrap()
            })
            .collect()
    }
}

impl From<&PayTubeTransaction> for SolanaInstruction {
    fn from(value: &PayTubeTransaction) -> Self {
        let PayTubeTransaction {
            mint,
            from,
            to,
            amount,
        } = value;
        if let Some(mint) = mint {
            // Insert SPL token transfer here.
            return SolanaInstruction::new_with_bytes(*mint, &[], vec![]);
        }
        system_instruction::transfer(from, to, *amount)
    }
}
