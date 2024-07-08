//! PayTube's "settler" component for settling the final ledgers across all
//! channel participants.
//!
//! When users are finished transacting, the resulting ledger is used to craft
//! a batch of transactions to settle all state changes to the base chain
//! (Solana).
//!
//! The interesting piece here is that there can be hundreds or thousands of
//! transactions across a handful of users, but only the resulting difference
//! between their balance when the channel opened and their balance when the
//! channel is about to close are needed to create the settlement transaction.

use {
    crate::transaction::PayTubeTransaction,
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        instruction::Instruction as SolanaInstruction, pubkey::Pubkey, signature::Keypair,
        signer::Signer, system_instruction, transaction::Transaction as SolanaTransaction,
    },
    solana_svm::transaction_processor::LoadAndExecuteSanitizedTransactionsOutput,
    std::collections::HashMap,
};

/// The key used for storing ledger entries.
///
/// Comprised of the mint (optional), the sender, then the recipient.
#[derive(PartialEq, Eq, Hash)]
struct LedgerKey {
    mint: Option<Pubkey>,
    from: Pubkey,
    to: Pubkey,
}

/// A ledger of PayTube transactions, used to deconstruct into base chain
/// transactions.
struct Ledger {
    ledger: HashMap<LedgerKey, u64>,
}

impl Ledger {
    fn new(
        paytube_transactions: &[PayTubeTransaction],
        svm_output: LoadAndExecuteSanitizedTransactionsOutput,
    ) -> Self {
        let mut ledger: HashMap<LedgerKey, u64> = HashMap::new();
        paytube_transactions
            .iter()
            .zip(svm_output.execution_results)
            .for_each(|(transaction, result)| {
                // Only append to the ledger if the PayTube transaction was
                // successful.
                //
                // TODO: Collapse redundant to -> from -> to movements.
                if result.was_executed_successfully() {
                    *ledger
                        .entry(LedgerKey {
                            mint: transaction.mint,
                            from: transaction.from,
                            to: transaction.to,
                        })
                        .or_default() += transaction.amount;
                }
            });
        Self { ledger }
    }

    fn generate_base_chain_instructions(&self) -> Vec<SolanaInstruction> {
        self.ledger
            .iter()
            .map(|(key, amount)| {
                if let Some(mint) = key.mint {
                    // Insert SPL token transfer here.
                    return SolanaInstruction::new_with_bytes(mint, &[], vec![]);
                }
                system_instruction::transfer(&key.from, &key.to, *amount)
            })
            .collect::<Vec<_>>()
    }
}

/// PayTube final transaction settler.
pub struct PayTubeSettler<'a> {
    rpc_client: &'a RpcClient,
}

impl<'a> PayTubeSettler<'a> {
    pub fn new(rpc_client: &'a RpcClient) -> Self {
        Self { rpc_client }
    }

    /// Settle the payment channel results to the Solana blockchain.
    pub fn process_settle(
        &self,
        paytube_transactions: &[PayTubeTransaction],
        svm_output: LoadAndExecuteSanitizedTransactionsOutput,
        keys: &[Keypair],
    ) {
        // Build the ledger from the processed PayTube transactions.
        let ledger = Ledger::new(paytube_transactions, svm_output);

        // Build the Solana instructions from the ledger.
        let instructions = ledger.generate_base_chain_instructions();

        // Send the transactions to the Solana blockchain.
        let recent_blockhash = self.rpc_client.get_latest_blockhash().unwrap();
        instructions.chunks(10).for_each(|chunk| {
            let transaction = SolanaTransaction::new_signed_with_payer(
                chunk,
                Some(&keys[0].pubkey()),
                keys,
                recent_blockhash,
            );
            self.rpc_client
                .send_and_confirm_transaction(&transaction)
                .unwrap();
        });
    }
}
