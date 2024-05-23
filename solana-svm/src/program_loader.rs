use {
    solana_program_runtime::{invoke_context::InvokeContext, solana_rbpf::elf::Executable},
    solana_sdk::pubkey::Pubkey,
};

/// Required plugin for loading Solana programs.
pub trait ProgramLoader {
    /// Load the executable for the program at the provided program ID.
    fn load_program(&self, program_id: &Pubkey) -> Option<Executable<InvokeContext<'static>>>;
}
