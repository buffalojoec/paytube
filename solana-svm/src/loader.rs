use {
    crate::transaction_processing_config::TransactionProcessingConfig,
    solana_program_runtime::{invoke_context::InvokeContext, solana_rbpf::elf::Executable},
    solana_sdk::{
        account::AccountSharedData,
        pubkey::Pubkey,
        sysvar::{Sysvar, SysvarId},
    },
};

/// The main customizable plugin for loading Solana accounts, programs, and
/// sysvars.
///
/// This plugin is required for transaction processing, but comes with a
/// generous handful of default implementations, to enhance bootstrapping.
///
/// Each method accepts a `TransactionProcessingConfig` object, which can be
/// used to customize the behavior of the loader.
pub trait Loader {
    /// Load the account at the provided address.
    fn load_account(
        &self,
        address: &Pubkey,
        config: &TransactionProcessingConfig,
    ) -> Option<AccountSharedData>;

    /// Load the executable for the program at the provided program ID.
    fn load_program(
        &self,
        program_id: &Pubkey,
        config: &TransactionProcessingConfig,
    ) -> Option<Executable<InvokeContext<'static>>>;

    /// Load the sysvar data for the provided sysvar type.
    fn load_sysvar<S: Sysvar + SysvarId>(&self, config: &TransactionProcessingConfig) -> Option<S>;
}
