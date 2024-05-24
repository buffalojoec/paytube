use {
    crate::nonce_info::NonceFull,
    solana_sdk::{
        rent_debits::RentDebits,
        transaction::Result,
        transaction_context::{IndexOfAccount, TransactionAccount},
    },
};

pub(crate) type TransactionRent = u64;
pub(crate) type TransactionProgramIndices = Vec<Vec<IndexOfAccount>>;
pub type TransactionLoadResult = Result<LoadedTransaction>;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LoadedTransaction {
    pub accounts: Vec<TransactionAccount>,
    pub program_indices: TransactionProgramIndices,
    pub nonce: Option<NonceFull>,
    pub rent: TransactionRent,
    pub rent_debits: RentDebits,
}
