use {
    lazy_static::lazy_static,
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey, system_program},
    solana_svm::{loader::Loader, transaction_processing_config::TransactionProcessingConfig},
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

/// SVM implementation of the `Loader` plugin trait.
impl Loader for PayTubeAccountLoader {
    fn load_account(
        &self,
        address: &Pubkey,
        _config: &TransactionProcessingConfig,
    ) -> Option<AccountSharedData> {
        match *address {
            ALICE => Some(ACCOUNTS[0].1.clone()),
            BOB => Some(ACCOUNTS[1].1.clone()),
            WILL => Some(ACCOUNTS[2].1.clone()),
            _ => None,
        }
    }

    // If we wanted to, PayTube could override any of the default implementations
    // for the rest of the trait, such as:
    //
    // * `account_matches_owner`
    // * `load_program`
    // * `load_sysvar` ...
    //
    // We could also attach a `SysvarCache` instance to the `PayTubeAccountLoader`
    // and override `vend_sysvar_cache` to vend the local sysvar cache.
    //
    // In the Agave validator, this implementation would be `Bank`.
}
