#![allow(unused)]

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

#[derive(PartialEq, Eq, Hash)]
struct LedgerKey {
    mint: Option<Pubkey>,
    from: Pubkey,
    to: Pubkey,
}

struct LedgerEntry {
    mint: Option<Pubkey>,
    from: Pubkey,
    to: Pubkey,
    amount: u64,
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
        let mut ledger: HashMap<LedgerKey, LedgerEntry> = HashMap::new();
        paytube_transactions
            .iter()
            .zip(svm_output.execution_results)
            .for_each(|(instruction, _result)| {
                let key = LedgerKey {
                    mint: instruction.mint,
                    from: instruction.from,
                    to: instruction.to,
                };
                if let Some(entry) = ledger.get_mut(&key) {
                    entry.amount += instruction.amount;
                } else {
                    let entry = LedgerEntry {
                        mint: instruction.mint,
                        from: instruction.from,
                        to: instruction.to,
                        amount: instruction.amount,
                    };
                    ledger.insert(key, entry);
                }
            });

        // Build the Solana instructions from the ledger.
        let instructions = ledger
            .iter()
            .map(|(key, entry)| {
                if let Some(mint) = key.mint {
                    // Insert SPL token transfer here.
                    return SolanaInstruction::new_with_bytes(mint, &[], vec![]);
                }
                system_instruction::transfer(&key.from, &key.to, entry.amount)
            })
            .collect::<Vec<_>>();

        // Send the transactions to the Solana blockchain.
        let recent_blockhash = self.rpc_client.get_latest_blockhash().unwrap();
        instructions.chunks(10).for_each(|chunk| {
            let mut transaction = SolanaTransaction::new_signed_with_payer(
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
