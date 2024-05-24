use {
    paytube_svm::{transaction::PayTubeTransaction, PayTubeChannel},
    solana_sdk::{
        account::AccountSharedData, epoch_schedule::EpochSchedule, pubkey::Pubkey,
        signature::Keypair, signer::Signer, system_program,
    },
    solana_test_validator::{TestValidator, TestValidatorGenesis},
};

const SLOTS_PER_EPOCH: u64 = 50;

struct TestValidatorContext {
    pub test_validator: TestValidator,
    pub payer: Keypair,
}

impl TestValidatorContext {
    fn start_with_accounts(accounts: Vec<(Pubkey, AccountSharedData)>) -> Self {
        solana_logger::setup();

        let epoch_schedule = EpochSchedule::custom(SLOTS_PER_EPOCH, SLOTS_PER_EPOCH, false);

        let (test_validator, payer) = TestValidatorGenesis::default()
            .epoch_schedule(epoch_schedule)
            .add_accounts(accounts)
            .start();

        Self {
            test_validator,
            payer,
        }
    }
}

fn system_account(lamports: u64) -> AccountSharedData {
    AccountSharedData::new(lamports, 0, &system_program::id())
}

#[test]
fn test_paytube() {
    let alice = Keypair::new();
    let bob = Keypair::new();
    let will = Keypair::new();

    let alice_pubkey = alice.pubkey();
    let bob_pubkey = bob.pubkey();
    let will_pubkey = will.pubkey();

    let accounts = vec![
        (alice_pubkey, system_account(10_000_000)),
        (bob_pubkey, system_account(10_000_000)),
        (will_pubkey, system_account(10_000_000)),
    ];

    let context = TestValidatorContext::start_with_accounts(accounts);
    let test_validator = &context.test_validator;
    let payer = context.payer.insecure_clone();

    let rpc_client = test_validator.get_rpc_client();

    let paytube_channel = PayTubeChannel::new(vec![payer, alice, bob, will], rpc_client);

    paytube_channel.process_paytube_transfers(&[
        // Alice -> Bob 2_000_000
        PayTubeTransaction {
            from: alice_pubkey,
            to: bob_pubkey,
            amount: 2_000_000,
            mint: None,
        },
        // Bob -> Will 5_000_000
        PayTubeTransaction {
            from: bob_pubkey,
            to: will_pubkey,
            amount: 5_000_000,
            mint: None,
        },
        // Alice -> Bob 2_000_000
        PayTubeTransaction {
            from: alice_pubkey,
            to: bob_pubkey,
            amount: 2_000_000,
            mint: None,
        },
        // Will -> Alice 1_000_000
        PayTubeTransaction {
            from: will_pubkey,
            to: alice_pubkey,
            amount: 1_000_000,
            mint: None,
        },
    ]);

    // Ledger:
    // Alice:   10_000_000 - 2_000_000 - 2_000_000 + 1_000_000  = 7_000_000
    // Bob:     10_000_000 + 2_000_000 - 5_000_000 + 2_000_000  = 9_000_000
    // Will:    10_000_000 + 5_000_000 - 1_000_000              = 14_000_000
    let rpc_client = test_validator.get_rpc_client();
    assert_eq!(rpc_client.get_balance(&alice_pubkey).unwrap(), 7_000_000);
    assert_eq!(rpc_client.get_balance(&bob_pubkey).unwrap(), 9_000_000);
    assert_eq!(rpc_client.get_balance(&will_pubkey).unwrap(), 14_000_000);
}
