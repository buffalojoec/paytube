//! PayTube's custom transaction format, tailored specifically for SOL or SPL
//! token transfers.
//!
//! Mostly for demonstration purposes, to show how projects may use completely
//! different transactions in their protocol, then convert the resulting state
//! transitions into the necessary transactions for the base chain - in this
//! case Solana.

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

/// A simple PayTube transaction. Transfers SPL tokens or SOL from one account
/// to another.
///
/// A `None` value for `mint` represents native SOL.
pub struct PayTubeTransaction {
    pub mint: Option<Pubkey>,
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
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
            // TODO: Insert SPL token transfer here.
            return SolanaInstruction::new_with_bytes(*mint, &[], vec![]);
        }
        system_instruction::transfer(from, to, *amount)
    }
}

impl From<&PayTubeTransaction> for SolanaTransaction {
    fn from(value: &PayTubeTransaction) -> Self {
        SolanaTransaction::new_with_payer(&[SolanaInstruction::from(value)], Some(&value.from))
    }
}

impl From<&PayTubeTransaction> for SolanaSanitizedTransaction {
    fn from(value: &PayTubeTransaction) -> Self {
        SolanaSanitizedTransaction::try_from_legacy_transaction(
            SolanaTransaction::from(value),
            &HashSet::new(),
        )
        .unwrap()
    }
}

/// Create a batch of Solana transactions, for the Solana SVM's transaction
/// processor, from a batch of PayTube instructions.
pub fn create_svm_transactions(
    paytube_transactions: &[PayTubeTransaction],
) -> Vec<SolanaSanitizedTransaction> {
    paytube_transactions
        .iter()
        .map(SolanaSanitizedTransaction::from)
        .collect()
}
