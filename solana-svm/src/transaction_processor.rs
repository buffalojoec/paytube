use {
    crate::{
        account_loader::AccountLoader,
        loaded_transaction::{LoadedTransaction, TransactionLoadResult},
        program_loader::ProgramLoader,
        sysvar_loader::SysvarLoader,
        transaction_error_metrics::TransactionErrorMetrics,
        transaction_processing_config::TransactionProcessingConfig,
        transaction_results::TransactionExecutionResult,
    },
    solana_program_runtime::{compute_budget::ComputeBudget, timings::ExecuteTimings},
    solana_sdk::{
        epoch_schedule::EpochSchedule, pubkey::Pubkey, transaction::SanitizedTransaction,
    },
    std::collections::HashSet,
};

pub struct LoadAndExecuteSanitizedTransactionsOutput {
    // Vector of results indicating whether a transaction was executed or could not
    // be executed. Note executed transactions can still have failed!
    pub execution_results: Vec<TransactionExecutionResult>,
    /// Error metrics for transactions that were processed.
    pub error_metrics: TransactionErrorMetrics,
    /// Timings for transaction batch execution.
    pub execute_timings: ExecuteTimings,
    // Vector of loaded transactions from transactions that were processed.
    pub loaded_transactions: Vec<TransactionLoadResult>,
}

/// The transaction processor.
///
/// A customizable isolated Solana transaction processing unit.
pub struct TransactionBatchProcessor<AL, PL, SL>
where
    AL: AccountLoader,
    PL: ProgramLoader,
    SL: SysvarLoader,
{
    /// Required plugin for loading Solana accounts.
    account_loader: AL,
    /// Required plugin for loading Solana programs.
    program_loader: PL,
    /// Required plugin for loading Solana sysvars.
    sysvar_loader: SL,
    /// Epoch schedule.
    epoch_schedule: EpochSchedule,
    /// Builtin programs to use in transaction processing.
    builtin_program_ids: HashSet<Pubkey>,
}

impl<AL, PL, SL> TransactionBatchProcessor<AL, PL, SL>
where
    AL: AccountLoader,
    PL: ProgramLoader,
    SL: SysvarLoader,
{
    /// Create a new transaction processor.
    pub fn new(
        account_loader: AL,
        program_loader: PL,
        sysvar_loader: SL,
        epoch_schedule: EpochSchedule,
        builtin_program_ids: HashSet<Pubkey>,
    ) -> Self {
        Self {
            account_loader,
            program_loader,
            sysvar_loader,
            epoch_schedule,
            builtin_program_ids,
        }
    }

    /// Main transaction processor API.
    ///
    /// Process a batch of sanitized Solana transactions.
    pub fn load_and_execute_sanitized_transactions(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        config: &TransactionProcessingConfig,
    ) -> LoadAndExecuteSanitizedTransactionsOutput {
        // ...
        unimplemented!()
    }

    fn execute_loaded_transaction(
        &self,
        sanitized_tx: &SanitizedTransaction,
        loaded_tx: &LoadedTransaction,
        config: &TransactionProcessingConfig,
        compute_budget: &ComputeBudget,
        execute_timings: &mut ExecuteTimings,
        error_metrics: &mut TransactionErrorMetrics,
    ) -> TransactionExecutionResult {
        // ...
        unimplemented!()
    }
}
