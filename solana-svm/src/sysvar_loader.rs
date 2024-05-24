use {
    solana_program_runtime::sysvar_cache::SysvarCache,
    solana_sdk::sysvar::{Sysvar, SysvarId},
};

/// Required plugin for loading Solana sysvars.
pub trait SysvarLoader {
    /// Load the sysvar data for the provided sysvar type.
    fn load_sysvar<S: Sysvar + SysvarId>(&self) -> Option<S>;

    /// Vend a Solana program-runtime-compatible `SysvarCache` instance.
    ///
    /// This function has a default implementation, but projects can override
    /// it if they want to employ their own sysvar caching mechanism.
    fn vend_sysvar_cache(&self) -> SysvarCache {
        use solana_sdk::{
            clock::Clock, epoch_rewards::EpochRewards, epoch_schedule::EpochSchedule, rent::Rent,
            slot_hashes::SlotHashes, stake_history::StakeHistory,
        };

        let mut sysvar_cache = SysvarCache::default();
        sysvar_cache.fill_missing_entries(|sysvar_id, set_sysvar| {
            if sysvar_id == &Clock::id() {
                let clock = self.load_sysvar::<Clock>();
                set_sysvar(&bincode::serialize(&clock).unwrap());
            }
            if sysvar_id == &EpochSchedule::id() {
                let epoch_schedule = self.load_sysvar::<EpochSchedule>();
                set_sysvar(&bincode::serialize(&epoch_schedule).unwrap());
            }
            if sysvar_id == &EpochRewards::id() {
                let epoch_rewards = self.load_sysvar::<EpochRewards>();
                set_sysvar(&bincode::serialize(&epoch_rewards).unwrap());
            }
            if sysvar_id == &Rent::id() {
                let rent = self.load_sysvar::<Rent>();
                set_sysvar(&bincode::serialize(&rent).unwrap());
            }
            if sysvar_id == &SlotHashes::id() {
                let slot_hashes = self.load_sysvar::<SlotHashes>();
                set_sysvar(&bincode::serialize(&slot_hashes).unwrap());
            }
            if sysvar_id == &StakeHistory::id() {
                let stake_history = self.load_sysvar::<StakeHistory>();
                set_sysvar(&bincode::serialize(&stake_history).unwrap());
            }
        });
        sysvar_cache
    }
}
