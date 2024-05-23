use {
    crate::{
        loaded_transaction::{LoadedTransaction, TransactionLoadResult},
        loader::Loader,
        transaction_error_metrics::TransactionErrorMetrics,
        transaction_processing_config::TransactionProcessingConfig,
        transaction_results::TransactionExecutionResult,
    },
    solana_program_runtime::{compute_budget::ComputeBudget, timings::ExecuteTimings},
    solana_sdk::transaction::SanitizedTransaction,
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
pub struct TransactionBatchProcessor;

impl TransactionBatchProcessor {
    /// Main transaction processor API.
    ///
    /// Process a batch of sanitized Solana transactions.
    ///
    /// Accepts a list of sanitized transactions, a `Loader` plugin, and the
    /// processing configuration.
    pub fn load_and_execute_sanitized_transactions<L: Loader>(
        sanitized_txs: &[SanitizedTransaction],
        loader: &L,
        config: &TransactionProcessingConfig,
    ) -> LoadAndExecuteSanitizedTransactionsOutput {
        // ...
        unimplemented!()
    }
}

fn execute_loaded_transaction<L: Loader>(
    sanitized_tx: &SanitizedTransaction,
    loader: &L,
    loaded_tx: &LoadedTransaction,
    compute_budget: &ComputeBudget,
    config: &TransactionProcessingConfig,
    execute_timings: &mut ExecuteTimings,
    error_metrics: &mut TransactionErrorMetrics,
) -> TransactionExecutionResult {
    // ...
    unimplemented!()
}
