#![allow(unused)]

use {
    crate::transaction::PayTubeTransaction, solana_sdk::pubkey::Pubkey,
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
pub struct PayTubeSettler {
    ledger: HashMap<LedgerKey, LedgerEntry>,
}

impl PayTubeSettler {
    pub fn new(
        paytube_transactions: &[PayTubeTransaction],
        svm_output: LoadAndExecuteSanitizedTransactionsOutput,
    ) -> Self {
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
        Self { ledger }
    }

    /// Settle the payment channel results to the Solana blockchain.
    pub fn process_settle(&self) {
        // This settlement could be done in a number of ways.
        // The transaction could easily be built from the ledger.
        // ```
        // use solana_sdk::{
        //     instruction::Instruction as SolanaInstruction, system_instruction,
        //     transaction::Transaction as SolanaTransaction,
        // };
        //
        // let instructions = self
        //     .ledger
        //     .entries()
        //     .iter()
        //     .map(|(key, entry)| {
        //         if let Some(mint) = key.mint {
        //             // Insert SPL token transfer here.
        //             return SolanaInstruction::new_with_bytes(mint, &[], vec![]);
        //         }
        //         system_instruction::transfer(key.from, key.to, entry.amount)
        //     })
        //     .collect::<Vec<_>>();
        //
        // instructions.chunks(10).for_each(|chunk| {
        //     let transaction = SolanaTransaction::new_with_payer(&chunk, None);
        //     //
        //     // Send the transaction to the Solana blockchain.
        //     //
        // });
        //```
    }
}
