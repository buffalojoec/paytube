use {
    lazy_static::lazy_static,
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey, system_program},
    solana_svm::account_loader::AccountLoader,
};

const ALICE: Pubkey = Pubkey::new_from_array([1; 32]);
const BOB: Pubkey = Pubkey::new_from_array([2; 32]);
const WILL: Pubkey = Pubkey::new_from_array([3; 32]);

lazy_static! {
    pub static ref ACCOUNTS: Vec<(Pubkey, AccountSharedData)> = [
        (
            ALICE,
            AccountSharedData::new(200_000_000, 0, &system_program::id())
        ),
        (
            BOB,
            AccountSharedData::new(200_000_000, 0, &system_program::id())
        ),
        (
            WILL,
            AccountSharedData::new(100_000_000, 0, &system_program::id())
        ),
    ]
    .to_vec();
}

#[derive(Default)]
pub struct PayTubeAccountLoader;

/// SVM implementation of the `AccountLoader` plugin trait.
impl AccountLoader for PayTubeAccountLoader {
    fn load_account(&self, address: &Pubkey) -> Option<AccountSharedData> {
        match *address {
            ALICE => Some(ACCOUNTS[0].1.clone()),
            BOB => Some(ACCOUNTS[1].1.clone()),
            WILL => Some(ACCOUNTS[2].1.clone()),
            _ => None,
        }
    }
}
