use {
    solana_client::rpc_client::RpcClient,
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey},
    solana_svm::loader::Loader,
    std::{collections::HashMap, sync::RwLock},
};

pub struct PayTubeAccountLoader<'a> {
    // A simple cache.
    cache: RwLock<HashMap<Pubkey, AccountSharedData>>,
    rpc_client: &'a RpcClient,
}

impl<'a> PayTubeAccountLoader<'a> {
    pub fn new(rpc_client: &'a RpcClient) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            rpc_client,
        }
    }
}

/// SVM implementation of the `Loader` plugin trait.
impl Loader for PayTubeAccountLoader<'_> {
    fn load_account(&self, address: &Pubkey) -> Option<AccountSharedData> {
        if let Some(account) = self.cache.read().unwrap().get(address) {
            return Some(account.clone());
        }

        let account: AccountSharedData = self.rpc_client.get_account(address).ok()?.into();
        self.cache
            .write()
            .unwrap()
            .insert(*address, account.clone());

        Some(account)
    }

    // If we wanted to, PayTube could override any of the default implementations
    // for the rest of the trait, such as:
    //
    // * `account_matches_owner`
    // * `load_program`
    // * `load_sysvar`
    //   ...
    //
    // We could also attach a `SysvarCache` instance to the `PayTubeAccountLoader`
    // and override `vend_sysvar_cache` to vend the local sysvar cache.
    //
    // In the Agave validator, this implementation would be `Bank`.
}
