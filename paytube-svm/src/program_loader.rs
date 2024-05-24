use {
    crate::account_loader::PayTubeAccountLoader,
    solana_bpf_loader_program::syscalls::create_program_runtime_environment_v1,
    solana_program_runtime::{
        compute_budget::ComputeBudget, invoke_context::InvokeContext, solana_rbpf::elf::Executable,
    },
    solana_sdk::{
        account::ReadableAccount,
        bpf_loader,
        bpf_loader_upgradeable::{self, get_program_data_address, UpgradeableLoaderState},
        feature_set::FeatureSet,
        pubkey::Pubkey,
    },
    solana_svm::{account_loader::AccountLoader, program_loader::ProgramLoader},
    std::sync::Arc,
};

pub struct PayTubeProgramLoader<'a> {
    /// Leverages the account loader to load program account data into an
    /// executable.
    account_loader: &'a PayTubeAccountLoader,
    compute_budget: &'a ComputeBudget,
    feature_set: &'a FeatureSet,
}

impl<'a> PayTubeProgramLoader<'a> {
    pub fn new(
        account_loader: &'a PayTubeAccountLoader,
        compute_budget: &'a ComputeBudget,
        feature_set: &'a FeatureSet,
    ) -> Self {
        Self {
            account_loader,
            compute_budget,
            feature_set,
        }
    }
}

/// SVM implementation of the `ProgramLoader` plugin trait.
impl ProgramLoader for PayTubeProgramLoader<'_> {
    fn load_program(&self, program_id: &Pubkey) -> Option<Executable<InvokeContext<'static>>> {
        if let Some(program_account) = self.account_loader.load_account(program_id) {
            let owner = program_account.owner();

            let program_runtime_environment = create_program_runtime_environment_v1(
                self.feature_set,
                self.compute_budget,
                /* reject_deployment_of_broken_elfs */ false,
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
                if let Some(programdata_account) =
                    self.account_loader.load_account(&programdata_address)
                {
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
}
