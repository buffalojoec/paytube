use {
    crate::{
        account_rent_state::RentState,
        loaded_transaction::{LoadedTransaction, TransactionRent},
        nonce_info::{NonceFull, NoncePartial},
        transaction_error_metrics::TransactionErrorMetrics,
        transaction_processing_config::TransactionProcessingConfig,
    },
    itertools::Itertools,
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1,
    solana_program_runtime::{
        compute_budget::ComputeBudget,
        compute_budget_processor::process_compute_budget_instructions,
        invoke_context::InvokeContext, solana_rbpf::elf::Executable, sysvar_cache::SysvarCache,
    },
    solana_sdk::{
        account::{Account, AccountSharedData, ReadableAccount, WritableAccount},
        bpf_loader,
        bpf_loader_upgradeable::{self, get_program_data_address, UpgradeableLoaderState},
        clock::Clock,
        epoch_rewards::EpochRewards,
        epoch_schedule::EpochSchedule,
        feature_set,
        message::SanitizedMessage,
        native_loader,
        nonce::State as NonceState,
        pubkey::Pubkey,
        rent::{Rent, RentDue},
        rent_collector::{RentCollector, RENT_EXEMPT_RENT_EPOCH},
        rent_debits::RentDebits,
        saturating_add_assign,
        slot_hashes::SlotHashes,
        stake_history::StakeHistory,
        sysvar::{self, instructions::construct_instructions_data, Sysvar, SysvarId},
        transaction::{self, TransactionError},
        transaction_context::IndexOfAccount,
    },
    solana_system_program::{get_system_account_kind, SystemAccountKind},
    std::{collections::HashSet, num::NonZeroUsize, sync::Arc},
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

    /// Determine whether or not an account is owned by one of the programs in
    /// the provided set.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to provide a more efficient implementation, such as
    /// checking account ownership without fully loading.
    fn account_matches_owners(
        &self,
        account: &Pubkey,
        owners: &[Pubkey],
        config: &TransactionProcessingConfig,
    ) -> bool {
        self.load_account(account, config)
            .map(|account| owners.contains(account.owner()))
            .unwrap_or(false)
    }

    /// Loads a set of transaction accounts and assesses the fee to the fee
    /// payer.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to provide a more efficient implementation, such as
    /// loading multiple accounts in parallel.
    fn load_transaction_accounts(
        &self,
        message: &SanitizedMessage,
        nonce: Option<&NoncePartial>,
        fee: u64,
        program_account_keys: &HashSet<Pubkey>,
        config: &TransactionProcessingConfig,
        error_metrics: &mut TransactionErrorMetrics,
    ) -> transaction::Result<LoadedTransaction> {
        let feature_set = config.feature_set;

        // There is no way to predict what program will execute without an error
        // If a fee can pay for execution then the program will be scheduled
        let mut validated_fee_payer = false;
        let mut tx_rent: TransactionRent = 0;
        let account_keys = message.account_keys();
        let mut accounts_found = Vec::with_capacity(account_keys.len());
        let mut rent_debits = RentDebits::default();
        let rent_collector = config.rent_collector;

        let requested_loaded_accounts_data_size_limit =
            get_requested_loaded_accounts_data_size_limit(message)?;
        let mut accumulated_accounts_data_size: usize = 0;

        let instruction_accounts = message
            .instructions()
            .iter()
            .flat_map(|instruction| &instruction.accounts)
            .unique()
            .collect::<Vec<&u8>>();

        let mut accounts = account_keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let mut account_found = true;
                #[allow(clippy::collapsible_else_if)]
                let account = if solana_sdk::sysvar::instructions::check_id(key) {
                    construct_instructions_account(message)
                } else {
                    let instruction_account = u8::try_from(i)
                        .map(|i| instruction_accounts.contains(&&i))
                        .unwrap_or(false);
                    let (account_size, mut account, rent) = if let Some(account_override) = config
                        .account_overrides
                        .and_then(|overrides| overrides.accounts.get(key))
                    {
                        (account_override.data().len(), account_override.clone(), 0)
                    } else if let Some(_program_id) = (!instruction_account
                        && !message.is_writable(i))
                    .then_some(())
                    .and_then(|_| program_account_keys.get(key))
                    {
                        self.load_account(key, config)
                            .map(|acct: AccountSharedData| (acct.data().len(), acct, 0))
                            .ok_or(TransactionError::AccountNotFound)?
                    } else {
                        self.load_account(key, config)
                            .map(|mut account| {
                                if message.is_writable(i) {
                                    if !feature_set
                                        .is_active(&feature_set::disable_rent_fees_collection::id())
                                    {
                                        let rent_due = rent_collector
                                            .collect_from_existing_account(key, &mut account)
                                            .rent_amount;

                                        (account.data().len(), account, rent_due)
                                    } else {
                                        // When rent fee collection is disabled, we won't collect
                                        // rent for any account. If there
                                        // are any rent paying accounts, their `rent_epoch` won't
                                        // change either. However, if the
                                        // account itself is rent-exempted but its `rent_epoch` is
                                        // not u64::MAX, we will set its
                                        // `rent_epoch` to u64::MAX. In such case, the behavior
                                        // stays the same as before.
                                        if account.rent_epoch() != RENT_EXEMPT_RENT_EPOCH
                                            && rent_collector.get_rent_due(
                                                account.lamports(),
                                                account.data().len(),
                                                account.rent_epoch(),
                                            ) == RentDue::Exempt
                                        {
                                            account.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                                        }
                                        (account.data().len(), account, 0)
                                    }
                                } else {
                                    (account.data().len(), account, 0)
                                }
                            })
                            .unwrap_or_else(|| {
                                account_found = false;
                                let mut default_account = AccountSharedData::default();
                                // All new accounts must be rent-exempt (enforced in
                                // Bank::execute_loaded_transaction).
                                // Currently, rent collection sets rent_epoch to u64::MAX, but
                                // initializing the account
                                // with this field already set would allow us to skip rent
                                // collection for these accounts.
                                default_account.set_rent_epoch(RENT_EXEMPT_RENT_EPOCH);
                                (default_account.data().len(), default_account, 0)
                            })
                    };
                    accumulate_and_check_loaded_account_data_size(
                        &mut accumulated_accounts_data_size,
                        account_size,
                        requested_loaded_accounts_data_size_limit,
                        error_metrics,
                    )?;

                    if i == 0 {
                        validate_fee_payer(
                            key,
                            &mut account,
                            i as IndexOfAccount,
                            error_metrics,
                            rent_collector,
                            fee,
                        )?;

                        validated_fee_payer = true;
                    }

                    tx_rent += rent;
                    rent_debits.insert(key, rent, account.lamports());

                    account
                };

                accounts_found.push(account_found);
                Ok((*key, account))
            })
            .collect::<transaction::Result<Vec<_>>>()?;

        if !validated_fee_payer {
            error_metrics.account_not_found += 1;
            return Err(TransactionError::AccountNotFound);
        }

        // Update nonce with fee-subtracted accounts
        let nonce = nonce.map(|nonce| {
            // SAFETY: The first accounts entry must be a validated fee payer because
            // validated_fee_payer must be true at this point.
            let (fee_payer_address, fee_payer_account) = accounts.first().unwrap();
            NonceFull::from_partial(
                nonce,
                fee_payer_address,
                fee_payer_account.clone(),
                &rent_debits,
            )
        });

        let builtins_start_index = accounts.len();
        let program_indices = message
            .instructions()
            .iter()
            .map(|instruction| {
                let mut account_indices = Vec::with_capacity(2);
                let mut program_index = instruction.program_id_index as usize;
                // This command may never return error, because the transaction is sanitized
                let (program_id, program_account) = accounts
                    .get(program_index)
                    .ok_or(TransactionError::ProgramAccountNotFound)?;
                if native_loader::check_id(program_id) {
                    return Ok(account_indices);
                }

                let account_found = accounts_found.get(program_index).unwrap_or(&true);
                if !account_found {
                    error_metrics.account_not_found += 1;
                    return Err(TransactionError::ProgramAccountNotFound);
                }

                if !program_account.executable() {
                    error_metrics.invalid_program_for_execution += 1;
                    return Err(TransactionError::InvalidProgramForExecution);
                }
                account_indices.insert(0, program_index as IndexOfAccount);
                let owner_id = program_account.owner();
                if native_loader::check_id(owner_id) {
                    return Ok(account_indices);
                }
                program_index = if let Some(owner_index) = accounts
                    .get(builtins_start_index..)
                    .ok_or(TransactionError::ProgramAccountNotFound)?
                    .iter()
                    .position(|(key, _)| key == owner_id)
                {
                    builtins_start_index.saturating_add(owner_index)
                } else {
                    let owner_index = accounts.len();
                    if let Some(owner_account) = self.load_account(owner_id, config) {
                        if !native_loader::check_id(owner_account.owner())
                            || !owner_account.executable()
                        {
                            error_metrics.invalid_program_for_execution += 1;
                            return Err(TransactionError::InvalidProgramForExecution);
                        }
                        accumulate_and_check_loaded_account_data_size(
                            &mut accumulated_accounts_data_size,
                            owner_account.data().len(),
                            requested_loaded_accounts_data_size_limit,
                            error_metrics,
                        )?;
                        accounts.push((*owner_id, owner_account));
                    } else {
                        error_metrics.account_not_found += 1;
                        return Err(TransactionError::ProgramAccountNotFound);
                    }
                    owner_index
                };
                account_indices.insert(0, program_index as IndexOfAccount);
                Ok(account_indices)
            })
            .collect::<transaction::Result<Vec<Vec<IndexOfAccount>>>>()?;

        Ok(LoadedTransaction {
            accounts,
            program_indices,
            nonce,
            rent: tx_rent,
            rent_debits,
        })
    }

    /// Load the executable for the program at the provided program ID.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to employ their own program loading mechanism, such as a
    /// JIT cache.
    fn load_program(
        &self,
        program_id: &Pubkey,
        config: &TransactionProcessingConfig,
    ) -> Option<Executable<InvokeContext<'static>>> {
        if let Some(program_account) = self.load_account(program_id, config) {
            let owner = program_account.owner();

            let program_runtime_environment = create_program_runtime_environment_v1(
                config.feature_set,
                &ComputeBudget::default(),
                false,
                false,
            )
            .unwrap();

            if bpf_loader::check_id(owner) {
                let programdata = program_account.data();
                let executable =
                    Executable::load(programdata, Arc::new(program_runtime_environment)).unwrap();
                return Some(executable);
            }

            if bpf_loader_upgradeable::check_id(owner) {
                let programdata_address = get_program_data_address(program_id);
                if let Some(programdata_account) = self.load_account(&programdata_address, config) {
                    let programdata = programdata_account
                        .data()
                        .get(UpgradeableLoaderState::size_of_programdata_metadata()..)?;
                    let executable =
                        Executable::load(programdata, Arc::new(program_runtime_environment))
                            .unwrap();
                    return Some(executable);
                }
            }
        }
        None
    }

    /// Load the sysvar data for the provided sysvar type.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to employ their own sysvar loading mechanism.
    fn load_sysvar<S: Sysvar + SysvarId>(&self, config: &TransactionProcessingConfig) -> Option<S> {
        self.load_account(&S::id(), config)
            .map(|sysvar_account| {
                let sysvar_data = sysvar_account.data();
                bincode::deserialize::<S>(sysvar_data).ok()
            })
            .unwrap_or(Some(S::default()))
    }

    /// Vend a Solana program-runtime-compatible `SysvarCache` instance.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to employ their own sysvar caching mechanism.
    fn vend_sysvar_cache(&self, config: &TransactionProcessingConfig) -> SysvarCache {
        let mut sysvar_cache = SysvarCache::default();
        sysvar_cache.fill_missing_entries(|sysvar_id, set_sysvar| {
            if sysvar_id == &Clock::id() {
                let clock = self.load_sysvar::<Clock>(config);
                set_sysvar(&bincode::serialize(&clock).unwrap());
            }
            if sysvar_id == &EpochSchedule::id() {
                let epoch_schedule = self.load_sysvar::<EpochSchedule>(config);
                set_sysvar(&bincode::serialize(&epoch_schedule).unwrap());
            }
            if sysvar_id == &EpochRewards::id() {
                let epoch_rewards = self.load_sysvar::<EpochRewards>(config);
                set_sysvar(&bincode::serialize(&epoch_rewards).unwrap());
            }
            if sysvar_id == &Rent::id() {
                let rent = self.load_sysvar::<Rent>(config);
                set_sysvar(&bincode::serialize(&rent).unwrap());
            }
            if sysvar_id == &SlotHashes::id() {
                let slot_hashes = self.load_sysvar::<SlotHashes>(config);
                set_sysvar(&bincode::serialize(&slot_hashes).unwrap());
            }
            if sysvar_id == &StakeHistory::id() {
                let stake_history = self.load_sysvar::<StakeHistory>(config);
                set_sysvar(&bincode::serialize(&stake_history).unwrap());
            }
        });
        sysvar_cache
    }
}

/// Total accounts data a transaction can load is limited to
///   if `set_tx_loaded_accounts_data_size` instruction is not activated or not
/// used, then     default value of 64MiB to not break anyone in Mainnet-beta
/// today   else
///     user requested loaded accounts size.
///     Note, requesting zero bytes will result transaction error
fn get_requested_loaded_accounts_data_size_limit(
    sanitized_message: &SanitizedMessage,
) -> transaction::Result<Option<NonZeroUsize>> {
    let compute_budget_limits =
        process_compute_budget_instructions(sanitized_message.program_instructions_iter())
            .unwrap_or_default();
    // sanitize against setting size limit to zero
    NonZeroUsize::new(
        usize::try_from(compute_budget_limits.loaded_accounts_bytes).unwrap_or_default(),
    )
    .map_or(
        Err(TransactionError::InvalidLoadedAccountsDataSizeLimit),
        |v| Ok(Some(v)),
    )
}

/// Accumulate loaded account data size into `accumulated_accounts_data_size`.
/// Returns TransactionErr::MaxLoadedAccountsDataSizeExceeded if
/// `requested_loaded_accounts_data_size_limit` is specified and
/// `accumulated_accounts_data_size` exceeds it.
fn accumulate_and_check_loaded_account_data_size(
    accumulated_loaded_accounts_data_size: &mut usize,
    account_data_size: usize,
    requested_loaded_accounts_data_size_limit: Option<NonZeroUsize>,
    error_metrics: &mut TransactionErrorMetrics,
) -> transaction::Result<()> {
    if let Some(requested_loaded_accounts_data_size) = requested_loaded_accounts_data_size_limit {
        saturating_add_assign!(*accumulated_loaded_accounts_data_size, account_data_size);
        if *accumulated_loaded_accounts_data_size > requested_loaded_accounts_data_size.get() {
            error_metrics.max_loaded_accounts_data_size_exceeded += 1;
            Err(TransactionError::MaxLoadedAccountsDataSizeExceeded)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

fn construct_instructions_account(message: &SanitizedMessage) -> AccountSharedData {
    AccountSharedData::from(Account {
        data: construct_instructions_data(&message.decompile_instructions()),
        owner: sysvar::id(),
        ..Account::default()
    })
}

/// Check whether the payer_account is capable of paying the fee. The
/// side effect is to subtract the fee amount from the payer_account
/// balance of lamports. If the payer_acount is not able to pay the
/// fee, the error_metrics is incremented, and a specific error is
/// returned.
pub fn validate_fee_payer(
    payer_address: &Pubkey,
    payer_account: &mut AccountSharedData,
    payer_index: IndexOfAccount,
    error_metrics: &mut TransactionErrorMetrics,
    rent_collector: &RentCollector,
    fee: u64,
) -> transaction::Result<()> {
    if payer_account.lamports() == 0 {
        error_metrics.account_not_found += 1;
        return Err(TransactionError::AccountNotFound);
    }
    let system_account_kind = get_system_account_kind(payer_account).ok_or_else(|| {
        error_metrics.invalid_account_for_fee += 1;
        TransactionError::InvalidAccountForFee
    })?;
    let min_balance = match system_account_kind {
        SystemAccountKind::System => 0,
        SystemAccountKind::Nonce => {
            // Should we ever allow a fees charge to zero a nonce account's
            // balance. The state MUST be set to uninitialized in that case
            rent_collector.rent.minimum_balance(NonceState::size())
        }
    };

    payer_account
        .lamports()
        .checked_sub(min_balance)
        .and_then(|v| v.checked_sub(fee))
        .ok_or_else(|| {
            error_metrics.insufficient_funds += 1;
            TransactionError::InsufficientFundsForFee
        })?;

    let payer_pre_rent_state = RentState::from_account(payer_account, &rent_collector.rent);
    payer_account
        .checked_sub_lamports(fee)
        .map_err(|_| TransactionError::InsufficientFundsForFee)?;

    let payer_post_rent_state = RentState::from_account(payer_account, &rent_collector.rent);
    RentState::check_rent_state_with_account(
        &payer_pre_rent_state,
        &payer_post_rent_state,
        payer_address,
        payer_account,
        payer_index,
    )
}
