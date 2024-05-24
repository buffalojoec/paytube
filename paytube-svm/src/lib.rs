//! PayTube. A simple SPL payment channel.
//!
//! PayTube is an SVM-based payment channel that allows two parties to exchange
//! tokens off-chain, without touching the blockchain. The channel is opened by
//! invoking the PayTube "VM", running on some arbitrary server. When
//! transacting has concluded, the channel is closed by submitting the final
//! payment ledger to Solana.
//!
//! The final ledger tracks debits and credits to all registered token accounts
//! or system accounts (native SOL) during the lifetime of a channel. It's then
//! used to to craft a batch of Solana transactions to submit to the network.
//!
//! Users opt-in to using a PayTube channel by "registering" their token
//! accounts to the channel. This is done by delegating a token account to the
//! PayTube on-chain program on Solana. This delegation is temporary, and
//! released immediately after channel settlement.
//!
//! *Registering and settling are not implemented in this example.*
//!
//! ```ignore
//! 
//! PayTube "VM"
//!
//!    Bob          Alice        Bob          Alice          Will
//!     |             |           |             |             |
//!     | --o--o--o-> |           | --o--o--o-> |             |
//!     |             |           |             | --o--o--o-> | <--- PayTube
//!     | <-o--o--o-- |           | <-o--o--o-- |             |    Transactions
//!     |             |           |             |             |
//!     | --o--o--o-> |           |     -----o--o--o----->    |
//!     |             |           |                           |
//!     | --o--o--o-> |           |     <----o--o--o------    |
//!
//!       \        /                  \         |         /
//!
//!         ------                           ------
//!        Alice: x                         Alice: x
//!        Bob:   x                         Bob:   x    <--- Solana Transaction
//!                                         Will:  x         with final ledgers
//!         ------                           ------
//!
//!           \\                               \\
//!            x                                x
//!
//!         Solana                           Solana     <--- Settled to Solana
//! ```
//!
//! The Solana SVM requires three plugins:
//!
//! * Account Loader
//! * Program Loader
//! * Sysvar Loader
//!
//! PayTube implements each of these plugins and provides them to a
//! `TransactionBatchProcessor` instance in order to leverage the Solana SVM
//! to process PayTube transactions.
