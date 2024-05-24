use {
    crate::nonce_info::{NonceFull, NonceInfo},
    solana_sdk::{
        inner_instruction::InnerInstructionsList,
        pubkey::Pubkey,
        rent_debits::RentDebits,
        transaction::{self, TransactionError},
        transaction_context::TransactionReturnData,
    },
    std::collections::HashSet,
};

pub struct TransactionResults {
    pub fee_collection_results: Vec<transaction::Result<()>>,
    pub execution_results: Vec<TransactionExecutionResult>,
    pub rent_debits: Vec<RentDebits>,
}

/// Type safe representation of a transaction execution attempt which
/// differentiates between a transaction that was executed (will be
/// committed to the ledger) and a transaction which wasn't executed
/// and will be dropped.
#[derive(Debug, Clone)]
pub enum TransactionExecutionResult {
    Executed {
        details: TransactionExecutionDetails,
        programs_modified_by_tx: HashSet<Pubkey>,
    },
    NotExecuted(TransactionError),
}

#[derive(Debug, Clone)]
pub struct TransactionExecutionDetails {
    pub status: transaction::Result<()>,
    pub log_messages: Option<Vec<String>>,
    pub inner_instructions: Option<InnerInstructionsList>,
    pub durable_nonce_fee: Option<DurableNonceFee>,
    pub return_data: Option<TransactionReturnData>,
    pub executed_units: u64,
    /// The change in accounts data len for this transaction.
    /// NOTE: This value is valid IFF `status` is `Ok`.
    pub accounts_data_len_delta: i64,
}

#[derive(Debug, Clone)]
pub enum DurableNonceFee {
    Valid(u64),
    Invalid,
}

impl From<&NonceFull> for DurableNonceFee {
    fn from(nonce: &NonceFull) -> Self {
        match nonce.lamports_per_signature() {
            Some(lamports_per_signature) => Self::Valid(lamports_per_signature),
            None => Self::Invalid,
        }
    }
}
