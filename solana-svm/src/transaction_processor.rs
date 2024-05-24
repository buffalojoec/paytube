use {
    crate::{
        account_loader::AccountLoader,
        loaded_transaction::{LoadedTransaction, TransactionLoadResult},
        message_processor::MessageProcessor,
        program_loader::ProgramLoader,
        sysvar_loader::SysvarLoader,
        transaction_account_state_info::TransactionAccountStateInfo,
        transaction_error_metrics::TransactionErrorMetrics,
        transaction_processing_config::TransactionProcessingConfig,
        transaction_results::{
            DurableNonceFee, TransactionExecutionDetails, TransactionExecutionResult,
        },
    },
    solana_measure::measure::Measure,
    solana_program_runtime::{
        compute_budget::ComputeBudget,
        compute_budget_processor::process_compute_budget_instructions,
        invoke_context::{EnvironmentConfig, InvokeContext},
        loaded_programs::{
            ForkGraph, ProgramCache, ProgramCacheEntry, ProgramCacheEntryOwner,
            ProgramCacheEntryType, ProgramCacheForTxBatch,
        },
        log_collector::LogCollector,
        timings::{ExecuteTimingType, ExecuteTimings},
    },
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount, PROGRAM_OWNERS},
        epoch_schedule::EpochSchedule,
        feature_set::{
            include_loaded_accounts_data_size_in_fee_calculation,
            remove_rounding_in_fee_calculation,
        },
        inner_instruction::{InnerInstruction, InnerInstructionsList},
        instruction::{CompiledInstruction, TRANSACTION_LEVEL_STACK_HEIGHT},
        message::SanitizedMessage,
        pubkey::Pubkey,
        saturating_add_assign,
        transaction::{SanitizedTransaction, TransactionError},
        transaction_context::{ExecutionRecord, TransactionContext},
    },
    std::{collections::HashSet, rc::Rc, sync::Arc},
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
        // Initialize metrics.
        let mut error_metrics = TransactionErrorMetrics::default();
        let mut execute_timings = ExecuteTimings::default();

        // Gather all program keys.
        let mut program_account_keys =
            self.filter_executable_program_accounts(sanitized_txs, PROGRAM_OWNERS);
        self.builtin_program_ids.iter().for_each(|id| {
            program_account_keys.insert(*id);
        });

        // Load the transactions.
        let mut load_time = Measure::start("accounts_load");
        let mut loaded_transactions = self.load_transactions(
            sanitized_txs,
            &program_account_keys,
            config,
            &mut error_metrics,
        );
        load_time.stop();

        // Execute the transactions.
        let mut execution_time = Measure::start("execution_time");

        let execution_results: Vec<TransactionExecutionResult> = loaded_transactions
            .iter_mut()
            .zip(sanitized_txs.iter())
            .map(|(load_result, tx)| match load_result {
                Err(e) => TransactionExecutionResult::NotExecuted(e.clone()),
                Ok(loaded_transaction) => match config.compute_budget {
                    Some(compute_budget) => self.execute_loaded_transaction(
                        tx,
                        loaded_transaction,
                        config,
                        compute_budget,
                        &mut execute_timings,
                        &mut error_metrics,
                    ),
                    None => {
                        let mut compute_budget_process_transaction_time =
                            Measure::start("compute_budget_process_transaction_time");
                        let maybe_compute_budget = ComputeBudget::try_from_instructions(
                            tx.message().program_instructions_iter(),
                        );
                        compute_budget_process_transaction_time.stop();

                        saturating_add_assign!(
                            execute_timings
                                .execute_accessories
                                .compute_budget_process_transaction_us,
                            compute_budget_process_transaction_time.as_us()
                        );

                        if let Err(err) = maybe_compute_budget {
                            return TransactionExecutionResult::NotExecuted(err);
                        }

                        self.execute_loaded_transaction(
                            tx,
                            loaded_transaction,
                            config,
                            &maybe_compute_budget.unwrap(),
                            &mut execute_timings,
                            &mut error_metrics,
                        )
                    }
                },
            })
            .collect();

        execution_time.stop();

        execute_timings.saturating_add_in_place(ExecuteTimingType::LoadUs, load_time.as_us());
        execute_timings
            .saturating_add_in_place(ExecuteTimingType::ExecuteUs, execution_time.as_us());

        LoadAndExecuteSanitizedTransactionsOutput {
            loaded_transactions,
            execution_results,
            error_metrics,
            execute_timings,
        }
    }

    fn execute_loaded_transaction(
        &self,
        sanitized_tx: &SanitizedTransaction,
        loaded_tx: &mut LoadedTransaction,
        config: &TransactionProcessingConfig,
        compute_budget: &ComputeBudget,
        execute_timings: &mut ExecuteTimings,
        error_metrics: &mut TransactionErrorMetrics,
    ) -> TransactionExecutionResult {
        let transaction_accounts = std::mem::take(&mut loaded_tx.accounts);

        fn transaction_accounts_lamports_sum(
            accounts: &[(Pubkey, AccountSharedData)],
            message: &SanitizedMessage,
        ) -> Option<u128> {
            let mut lamports_sum = 0u128;
            for i in 0..message.account_keys().len() {
                let (_, account) = accounts.get(i)?;
                lamports_sum = lamports_sum.checked_add(u128::from(account.lamports()))?;
            }
            Some(lamports_sum)
        }

        let lamports_before_tx =
            transaction_accounts_lamports_sum(&transaction_accounts, sanitized_tx.message())
                .unwrap_or(0);

        // These are shams to be able to create an `InvokeContext` instance.
        let mut shammed_program_cache = ProgramCache::<ShammedForkGraph>::new(0, 0);
        let mut program_cache_for_tx_batch = ProgramCacheForTxBatch::default();
        let mut programs_modified_by_tx = ProgramCacheForTxBatch::default();
        // Back-fill the local cache instance with loaded programs for the transaction.
        for program_indices in loaded_tx.program_indices.iter() {
            for index in program_indices.iter() {
                let (program_id, program_account) =
                    transaction_accounts.get(*index as usize).unwrap();
                let account_owner = ProgramCacheEntryOwner::try_from(program_account.owner())
                    .expect("Invalid program owner");
                if let Some(executable) = self.program_loader.load_program(program_id) {
                    program_cache_for_tx_batch.replenish(
                        *program_id,
                        Arc::new(ProgramCacheEntry {
                            account_owner,
                            program: ProgramCacheEntryType::Loaded(executable),
                            ..Default::default()
                        }),
                    );
                }
            }
        }

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            config.rent_collector.rent.clone(),
            compute_budget.max_invoke_stack_height,
            compute_budget.max_instruction_trace_length,
        );
        #[cfg(debug_assertions)]
        transaction_context.set_signature(sanitized_tx.signature());

        let pre_account_state_info = TransactionAccountStateInfo::new(
            &config.rent_collector.rent,
            &transaction_context,
            sanitized_tx.message(),
        );

        let log_collector = if config.recording_config.enable_log_recording {
            match config.log_messages_bytes_limit {
                None => Some(LogCollector::new_ref()),
                Some(log_messages_bytes_limit) => Some(LogCollector::new_ref_with_limit(Some(
                    log_messages_bytes_limit,
                ))),
            }
        } else {
            None
        };

        let blockhash = config.blockhash;
        let lamports_per_signature = config.lamports_per_signature;

        let mut executed_units = 0u64;
        let sysvar_cache = self.sysvar_loader.vend_sysvar_cache();

        let mut invoke_context = InvokeContext::new(
            &mut transaction_context,
            &program_cache_for_tx_batch,
            EnvironmentConfig::new(
                blockhash,
                Arc::new(config.feature_set.clone()), // Yuck!
                lamports_per_signature,
                &sysvar_cache,
            ),
            log_collector.clone(),
            config.compute_budget.cloned().unwrap_or_default(), // Double-Yuck!
            &mut programs_modified_by_tx,
        );

        let mut process_message_time = Measure::start("process_message_time");
        let process_result = MessageProcessor::process_message(
            sanitized_tx.message(),
            &loaded_tx.program_indices,
            &mut invoke_context,
            execute_timings,
            &mut executed_units,
        );
        process_message_time.stop();

        drop(invoke_context);

        saturating_add_assign!(
            execute_timings.execute_accessories.process_message_us,
            process_message_time.as_us()
        );

        let mut status = process_result
            .and_then(|info| {
                let post_account_state_info = TransactionAccountStateInfo::new(
                    &config.rent_collector.rent,
                    &transaction_context,
                    sanitized_tx.message(),
                );
                TransactionAccountStateInfo::verify_changes(
                    &pre_account_state_info,
                    &post_account_state_info,
                    &transaction_context,
                )
                .map(|_| info)
            })
            .map_err(|err| {
                match err {
                    TransactionError::InvalidRentPayingAccount
                    | TransactionError::InsufficientFundsForRent { .. } => {
                        error_metrics.invalid_rent_paying_account += 1;
                    }
                    TransactionError::InvalidAccountIndex => {
                        error_metrics.invalid_account_index += 1;
                    }
                    _ => {
                        error_metrics.instruction_error += 1;
                    }
                }
                err
            });

        let log_messages: Option<Vec<String>> = log_collector.and_then(|log_collector| {
            Rc::try_unwrap(log_collector)
                .map(|log_collector| log_collector.into_inner().into_messages())
                .ok()
        });

        let inner_instructions = if config.recording_config.enable_cpi_recording {
            Some(inner_instructions_list_from_instruction_trace(
                &transaction_context,
            ))
        } else {
            None
        };

        let ExecutionRecord {
            accounts,
            return_data,
            touched_account_count,
            accounts_resize_delta: accounts_data_len_delta,
        } = transaction_context.into();

        if status.is_ok()
            && transaction_accounts_lamports_sum(&accounts, sanitized_tx.message())
                .filter(|lamports_after_tx| lamports_before_tx == *lamports_after_tx)
                .is_none()
        {
            status = Err(TransactionError::UnbalancedTransaction);
        }
        let status = status.map(|_| ());

        loaded_tx.accounts = accounts;
        saturating_add_assign!(
            execute_timings.details.total_account_count,
            loaded_tx.accounts.len() as u64
        );
        saturating_add_assign!(
            execute_timings.details.changed_account_count,
            touched_account_count
        );

        let return_data = if config.recording_config.enable_return_data_recording
            && !return_data.data.is_empty()
        {
            Some(return_data)
        } else {
            None
        };

        // Now collapse the shammed `programs_modified_by_tx` into the
        // `HashSet<Pubkey>` this implementation expects.
        // This is a bit of a hack, but it's the only way to publicly access
        // a program cache's entries.
        shammed_program_cache.merge(&programs_modified_by_tx);
        let programs_modified_by_tx = shammed_program_cache
            .get_flattened_entries(true, true)
            .iter()
            .map(|(key, _)| *key)
            .collect();

        TransactionExecutionResult::Executed {
            details: TransactionExecutionDetails {
                status,
                log_messages,
                inner_instructions,
                durable_nonce_fee: loaded_tx.nonce.as_ref().map(DurableNonceFee::from),
                return_data,
                executed_units,
                accounts_data_len_delta,
            },
            programs_modified_by_tx,
        }
    }

    fn filter_executable_program_accounts(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        program_owners: &[Pubkey],
    ) -> HashSet<Pubkey> {
        sanitized_txs
            .iter()
            .flat_map(|tx| {
                tx.message()
                    .account_keys()
                    .iter()
                    .filter(|key| {
                        self.account_loader
                            .account_matches_owners(key, program_owners)
                    })
                    .copied()
            })
            .collect()
    }

    fn load_transactions(
        &self,
        sanitized_txs: &[SanitizedTransaction],
        program_account_keys: &HashSet<Pubkey>,
        config: &TransactionProcessingConfig,
        error_metrics: &mut TransactionErrorMetrics,
    ) -> Vec<TransactionLoadResult> {
        let feature_set = config.feature_set;
        sanitized_txs
            .iter()
            .map(|tx| {
                let message = tx.message();
                let fee = config.fee_structure.calculate_fee(
                    message,
                    config.lamports_per_signature,
                    &process_compute_budget_instructions(message.program_instructions_iter())
                        .unwrap_or_default()
                        .into(),
                    feature_set
                        .is_active(&include_loaded_accounts_data_size_in_fee_calculation::id()),
                    feature_set.is_active(&remove_rounding_in_fee_calculation::id()),
                );
                self.account_loader.load_transaction_accounts(
                    message,
                    None, // Nonce omitted for now.
                    fee,
                    program_account_keys,
                    config,
                    error_metrics,
                )
            })
            .collect()
    }
}

/// Extract the InnerInstructionsList from a TransactionContext
fn inner_instructions_list_from_instruction_trace(
    transaction_context: &TransactionContext,
) -> InnerInstructionsList {
    debug_assert!(transaction_context
        .get_instruction_context_at_index_in_trace(0)
        .map(|instruction_context| instruction_context.get_stack_height()
            == TRANSACTION_LEVEL_STACK_HEIGHT)
        .unwrap_or(true));
    let mut outer_instructions = Vec::new();
    for index_in_trace in 0..transaction_context.get_instruction_trace_length() {
        if let Ok(instruction_context) =
            transaction_context.get_instruction_context_at_index_in_trace(index_in_trace)
        {
            let stack_height = instruction_context.get_stack_height();
            if stack_height == TRANSACTION_LEVEL_STACK_HEIGHT {
                outer_instructions.push(Vec::new());
            } else if let Some(inner_instructions) = outer_instructions.last_mut() {
                let stack_height = u8::try_from(stack_height).unwrap_or(u8::MAX);
                let instruction = CompiledInstruction::new_from_raw_parts(
                    instruction_context
                        .get_index_of_program_account_in_transaction(
                            instruction_context
                                .get_number_of_program_accounts()
                                .saturating_sub(1),
                        )
                        .unwrap_or_default() as u8,
                    instruction_context.get_instruction_data().to_vec(),
                    (0..instruction_context.get_number_of_instruction_accounts())
                        .map(|instruction_account_index| {
                            instruction_context
                                .get_index_of_instruction_account_in_transaction(
                                    instruction_account_index,
                                )
                                .unwrap_or_default() as u8
                        })
                        .collect(),
                );
                inner_instructions.push(InnerInstruction {
                    instruction,
                    stack_height,
                });
            } else {
                debug_assert!(false);
            }
        } else {
            debug_assert!(false);
        }
    }
    outer_instructions
}

// Shammed fork graph for the shammed program cache for the `InvokeContext`...
struct ShammedForkGraph;
impl ForkGraph for ShammedForkGraph {
    fn relationship(
        &self,
        _a: solana_sdk::clock::Slot,
        _b: solana_sdk::clock::Slot,
    ) -> solana_program_runtime::loaded_programs::BlockRelation {
        todo!("Sham!")
    }
}
