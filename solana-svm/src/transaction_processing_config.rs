use {
    solana_program_runtime::compute_budget::ComputeBudget,
    solana_sdk::{
        account::AccountSharedData, clock::Slot, feature_set::FeatureSet, fee::FeeStructure,
        hash::Hash, pubkey::Pubkey, rent_collector::RentCollector,
    },
    std::collections::HashMap,
};

/// Encapsulates overridden accounts, typically used for transaction simulation.
pub struct AccountOverrides {
    pub accounts: HashMap<Pubkey, AccountSharedData>,
}

/// Configuration of the recording capabilities for transaction execution.
pub struct ExecutionRecordingConfig {
    pub enable_cpi_recording: bool,
    pub enable_log_recording: bool,
    pub enable_return_data_recording: bool,
}

/// Configurations for processing transactions.
pub struct TransactionProcessingConfig<'a> {
    /// Encapsulates overridden accounts, typically used for transaction
    /// simulation.
    pub account_overrides: Option<&'a AccountOverrides>,
    /// The blockhash to use.
    pub blockhash: Hash,
    /// The compute budget to use.
    pub compute_budget: Option<&'a ComputeBudget>,
    /// The feature set to use.
    pub feature_set: &'a FeatureSet,
    /// The fee structure to use.
    pub fee_structure: &'a FeeStructure,
    /// Lamports per signature to charge the fee payer.
    pub lamports_per_signature: u64,
    /// The maximum number of bytes that log messages can consume.
    pub log_messages_bytes_limit: Option<usize>,
    /// Whether to limit the number of programs loaded for the transaction
    /// batch.
    pub limit_to_load_programs: bool,
    /// Recording capabilities for transaction execution.
    pub recording_config: ExecutionRecordingConfig,
    /// The rent collector to use.
    pub rent_collector: &'a RentCollector,
    /// The slot to use.
    pub slot: Slot,
}
