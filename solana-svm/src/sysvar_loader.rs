use solana_sdk::sysvar::{Sysvar, SysvarId};

/// Required plugin for loading Solana sysvars.
pub trait SysvarLoader {
    /// Load the sysvar data for the provided sysvar type.
    fn load_sysvar<S: Sysvar + SysvarId>(&self) -> Option<S>;
}
