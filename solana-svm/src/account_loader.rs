use solana_sdk::{account::AccountSharedData, pubkey::Pubkey};

/// Required plugin for loading Solana accounts.
pub trait AccountLoader {
    /// Load the account at the provided address.
    fn load_account(&self, address: &Pubkey) -> Option<AccountSharedData>;
}
