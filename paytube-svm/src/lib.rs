//! PayTube. A simple SPL payment channel.
//!
//! PayTube is an SVM-based payment channel that allows two parties to exchange
//! tokens off-chain, without touching the blockchain. The channel is opened by
//! invoking the PayTube "VM", running on some arbitrary server. When
//! transacting has concluded, the channel is closed by submitting the final
//! payment ledger to Solana.
//!
//! The final ledger tracks debits and credits to all registered token accounts
//! or system accounts (native SOL) during the lifetime of a channel. It's then
//! used to to craft a batch of Solana transactions to submit to the network.
//!
//! Users opt-in to using a PayTube channel by "registering" their token
//! accounts to the channel. This is done by delegating a token account to the
//! PayTube on-chain program on Solana. This delegation is temporary, and
//! released immediately after channel settlement.
//!
//! *Registering and settling are not implemented in this example.*
//!
//! ```ignore
//! 
//! PayTube "VM"
//!
//!    Bob          Alice        Bob          Alice          Will
//!     |             |           |             |             |
//!     | --o--o--o-> |           | --o--o--o-> |             |
//!     |             |           |             | --o--o--o-> | <--- PayTube
//!     | <-o--o--o-- |           | <-o--o--o-- |             |    Transactions
//!     |             |           |             |             |
//!     | --o--o--o-> |           |     -----o--o--o----->    |
//!     |             |           |                           |
//!     | --o--o--o-> |           |     <----o--o--o------    |
//!
//!       \        /                  \         |         /
//!
//!         ------                           ------
//!        Alice: x                         Alice: x
//!        Bob:   x                         Bob:   x    <--- Solana Transaction
//!                                         Will:  x         with final ledgers
//!         ------                           ------
//!
//!           \\                               \\
//!            x                                x
//!
//!         Solana                           Solana     <--- Settled to Solana
//! ```
//!
//! The Solana SVM requires three plugins:
//!
//! * Account Loader
//! * Program Loader
//! * Sysvar Loader
//!
//! PayTube implements each of these plugins and provides them to a
//! `TransactionBatchProcessor` instance in order to leverage the Solana SVM
//! to process PayTube transactions.

mod account_loader;
mod program_loader;
mod settler;
mod sysvar_loader;
pub mod transaction;

use {
    crate::{
        account_loader::PayTubeAccountLoader, program_loader::PayTubeProgramLoader,
        settler::PayTubeSettler, sysvar_loader::PayTubeSysvarLoader,
        transaction::PayTubeTransaction,
    },
    solana_client::rpc_client::RpcClient,
    solana_program_runtime::compute_budget::ComputeBudget,
    solana_sdk::{
        feature_set::FeatureSet, fee::FeeStructure, hash::Hash, rent_collector::RentCollector,
        signature::Keypair,
    },
    solana_svm::{
        transaction_processing_config::{ExecutionRecordingConfig, TransactionProcessingConfig},
        transaction_processor::TransactionBatchProcessor,
    },
    std::collections::HashSet,
};

pub struct PayTubeChannel {
    /// I think you know why this is a bad idea...
    keys: Vec<Keypair>,
    rpc_client: RpcClient,
}

impl PayTubeChannel {
    pub fn new(keys: Vec<Keypair>, rpc_client: RpcClient) -> Self {
        Self { keys, rpc_client }
    }

    /// The PayTube API. Processes a batch of PayTube transactions.
    ///
    /// Obviously this is a very simple implementation, but one could imagine
    /// a more complex service that employs custom functionality, such as:
    ///
    /// * Increased throughput for individual P2P transfers.
    /// * Custom Solana transaction ordering (e.g. MEV).
    ///
    /// The general scaffold of the PayTube API would remain the same.
    pub fn process_paytube_transfers(&self, transactions: &[PayTubeTransaction]) {
        // PayTube default configs.
        let compute_budget = ComputeBudget::default();
        let feature_set = FeatureSet::all_enabled();
        let fee_structure = FeeStructure::default();
        let rent_collector = RentCollector::default();

        // Loaders.
        let account_loader = PayTubeAccountLoader::new(&self.rpc_client);
        let program_loader =
            PayTubeProgramLoader::new(&account_loader, &compute_budget, &feature_set);
        let sysvar_loader = PayTubeSysvarLoader::new(&account_loader);

        // Transaction batch processor.
        let transaction_processor = TransactionBatchProcessor::new(
            &account_loader,
            &program_loader,
            &sysvar_loader,
            HashSet::default(),
        );

        // The default PayTube transaction processing config for Solana SVM.
        let processing_config = TransactionProcessingConfig {
            account_overrides: None,
            blockhash: Hash::default(),
            compute_budget: Some(&compute_budget),
            feature_set: &feature_set,
            fee_structure: &fee_structure,
            lamports_per_signature: fee_structure.lamports_per_signature,
            log_messages_bytes_limit: None,
            limit_to_load_programs: false,
            recording_config: ExecutionRecordingConfig {
                enable_cpi_recording: false,
                enable_log_recording: false,
                enable_return_data_recording: false,
            },
            rent_collector: &rent_collector,
            slot: 0,
        };

        // 1. Convert to a Solana SVM transaction batch.
        let svm_transactions = PayTubeTransaction::create_svm_transactions(transactions);

        // 2. Process transactions with the Solana SVM.
        let results = transaction_processor
            .load_and_execute_sanitized_transactions(&svm_transactions, &processing_config);

        // 3. Convert results into `PayTubeSettler`.
        let settler = PayTubeSettler::new(&self.rpc_client);

        // 4. Submit to Solana network.
        settler.process_settle(transactions, results, &self.keys);
    }
}
