use solana_sdk::{
    account::AccountSharedData,
    pubkey::Pubkey,
    rent_debits::RentDebits,
    transaction::Result,
    transaction_context::{IndexOfAccount, TransactionAccount},
};

pub(crate) type TransactionRent = u64;
pub(crate) type TransactionProgramIndices = Vec<Vec<IndexOfAccount>>;
pub type TransactionLoadResult = Result<LoadedTransaction>;

/// Holds limited nonce info available during transaction checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoncePartial {
    address: Pubkey,
    account: AccountSharedData,
}

/// Holds fee subtracted nonce info.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NonceFull {
    address: Pubkey,
    account: AccountSharedData,
    fee_payer_account: Option<AccountSharedData>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct LoadedTransaction {
    pub accounts: Vec<TransactionAccount>,
    pub program_indices: TransactionProgramIndices,
    pub nonce: Option<NonceFull>,
    pub rent: TransactionRent,
    pub rent_debits: RentDebits,
}
