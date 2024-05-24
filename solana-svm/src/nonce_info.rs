use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    nonce_account,
    pubkey::Pubkey,
    rent_debits::RentDebits,
};

pub trait NonceInfo {
    fn address(&self) -> &Pubkey;
    fn account(&self) -> &AccountSharedData;
    fn lamports_per_signature(&self) -> Option<u64>;
    fn fee_payer_account(&self) -> Option<&AccountSharedData>;
}

/// Holds limited nonce info available during transaction checks
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoncePartial {
    address: Pubkey,
    account: AccountSharedData,
}

impl NoncePartial {
    pub fn new(address: Pubkey, account: AccountSharedData) -> Self {
        Self { address, account }
    }
}

impl NonceInfo for NoncePartial {
    fn address(&self) -> &Pubkey {
        &self.address
    }
    fn account(&self) -> &AccountSharedData {
        &self.account
    }
    fn lamports_per_signature(&self) -> Option<u64> {
        nonce_account::lamports_per_signature_of(&self.account)
    }
    fn fee_payer_account(&self) -> Option<&AccountSharedData> {
        None
    }
}

/// Holds fee subtracted nonce info
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NonceFull {
    address: Pubkey,
    account: AccountSharedData,
    fee_payer_account: Option<AccountSharedData>,
}

impl NonceFull {
    pub fn new(
        address: Pubkey,
        account: AccountSharedData,
        fee_payer_account: Option<AccountSharedData>,
    ) -> Self {
        Self {
            address,
            account,
            fee_payer_account,
        }
    }
    pub fn from_partial(
        partial: &NoncePartial,
        fee_payer_address: &Pubkey,
        mut fee_payer_account: AccountSharedData,
        rent_debits: &RentDebits,
    ) -> Self {
        let rent_debit = rent_debits.get_account_rent_debit(fee_payer_address);
        fee_payer_account.set_lamports(fee_payer_account.lamports().saturating_add(rent_debit));

        let nonce_address = *partial.address();
        if *fee_payer_address == nonce_address {
            Self::new(nonce_address, fee_payer_account, None)
        } else {
            Self::new(
                nonce_address,
                partial.account().clone(),
                Some(fee_payer_account),
            )
        }
    }
}

impl NonceInfo for NonceFull {
    fn address(&self) -> &Pubkey {
        &self.address
    }
    fn account(&self) -> &AccountSharedData {
        &self.account
    }
    fn lamports_per_signature(&self) -> Option<u64> {
        nonce_account::lamports_per_signature_of(&self.account)
    }
    fn fee_payer_account(&self) -> Option<&AccountSharedData> {
        self.fee_payer_account.as_ref()
    }
}
