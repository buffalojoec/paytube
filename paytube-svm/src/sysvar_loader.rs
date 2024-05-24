use {
    crate::account_loader::PayTubeAccountLoader,
    solana_sdk::{account::ReadableAccount, sysvar::Sysvar},
    solana_svm::{account_loader::AccountLoader, sysvar_loader::SysvarLoader},
};

pub struct PayTubeSysvarLoader<'a> {
    /// Leverages the account loader to load sysvar data from accounts.
    account_loader: &'a PayTubeAccountLoader<'a>,
}

impl<'a> PayTubeSysvarLoader<'a> {
    pub fn new(account_loader: &'a PayTubeAccountLoader) -> Self {
        Self { account_loader }
    }
}

/// SVM implementation of the `SysvarLoader` plugin trait.
impl SysvarLoader for PayTubeSysvarLoader<'_> {
    fn load_sysvar<S: Sysvar>(&self) -> Option<S> {
        self.account_loader
            .load_account(&S::id())
            .map(|sysvar_account| {
                let sysvar_data = sysvar_account.data();
                bincode::deserialize::<S>(sysvar_data).ok()
            })
            .unwrap_or(Some(S::default()))
    }
}
