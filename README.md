# PayTube

A demonstration of a Solana SVM-based payment channel for SPL tokens.

The `solana-svm` crate is a reimplemented version of Agave's `solana-svm`
crate.

The `paytube-svm` crate is the PayTube channel itself, leveraging the Solana
SVM and also providing its own implementations for the SVM API.

## Reimplementing Solana SVM

Below is a diagram of the current `solana-svm` crate's
`TransactionBatchProcessor` API.

![v1](./doc/tx_processor_api_v1.jpg)

Below is a diagram of the proposed new API, as implemented here in this
repository under `solana-svm`.

![v2](./doc/tx_processor_api_v2.jpg)
