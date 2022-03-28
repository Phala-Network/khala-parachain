mod pegged {
    /// mint pegged token
    /// triggered by deposit at the original token vault
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Mint {
        #[prost(bytes="vec", tag="1")]
        pub token: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes="vec", tag="2")]
        pub account: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes="vec", tag="3")]
        pub amount: ::prost::alloc::vec::Vec<u8>,
        /// depositor defines the account address that made deposit at the original token chain,
        /// or the account address that burned tokens at another remote chain
        /// Not applicable to governance-triggered mints.
        #[prost(bytes="vec", tag="4")]
        pub depositor: ::prost::alloc::vec::Vec<u8>,
        /// ref_chain_id defines the reference chain ID, taking values of:
        /// 1. The common case: the chain ID on which the corresponding deposit or burn happened;
        /// 2. Governance-triggered mint: the chain ID on which the minting will happen.
        #[prost(uint64, tag="5")]
        pub ref_chain_id: u64,
        /// ref_id defines a unique reference ID, taking values of:
        /// 1. The common case of deposit/burn-mint: the deposit or burn ID;
        /// 2. Governance-triggered mint: ID as needed.
        #[prost(bytes="vec", tag="6")]
        pub ref_id: ::prost::alloc::vec::Vec<u8>,
    }
    /// withdraw locked original tokens
    /// triggered by burn at the pegged token bridge
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Withdraw {
        #[prost(bytes="vec", tag="1")]
        pub token: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes="vec", tag="2")]
        pub receiver: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes="vec", tag="3")]
        pub amount: ::prost::alloc::vec::Vec<u8>,
        /// burn_account defines the account that burned the pegged token.
        /// Not applicable to fee claims and governance-triggered withdrawals.
        #[prost(bytes="vec", tag="4")]
        pub burn_account: ::prost::alloc::vec::Vec<u8>,
        /// ref_chain_id defines the reference chain ID, taking values of:
        /// 1. The common case of burn-withdraw: the chain ID on which the corresponding burn happened;
        /// 2. Pegbridge fee claim: zero / Not applicable;
        /// 3. Other governance-triggered withdrawals: the chain ID on which the withdrawal will happen.
        #[prost(uint64, tag="5")]
        pub ref_chain_id: u64,
        /// ref_id defines a unique reference ID, taking values of:
        /// 1. The common case of burn-withdraw: the burn ID;
        /// 2. Pegbridge fee claim: a per-account nonce;
        /// 3. Refund for wrong deposit: the deposit ID;
        /// 4. Governance-triggered withdrawal: ID as needed.
        #[prost(bytes="vec", tag="6")]
        pub ref_id: ::prost::alloc::vec::Vec<u8>,
    }
}
