use {
    solana_client::rpc_client::RpcClient,
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey},
    solana_svm::account_loader::AccountLoader,
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

/// SVM implementation of the `AccountLoader` plugin trait.
impl AccountLoader for PayTubeAccountLoader<'_> {
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
}
