#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

pub mod merkle_tree;

use ink_lang as ink;

#[ink::contract]
pub mod mixer {
    use super::*;
    use ink_prelude::vec::Vec;
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use poseidon::poseidon::PoseidonRef;
    use protocol_ink_lib::keccak::Keccak256;
    use protocol_ink_lib::utils::{is_account_id_zero, truncate_and_pad};
    use protocol_ink_lib::zeroes::zeroes;
    use scale::Encode;
    use verifier::MixerVerifierRef;

    use openbrush::contracts::psp22::*;
    use openbrush::contracts::traits::psp22::PSP22;
    use openbrush::traits::Storage;

    pub const ROOT_HISTORY_SIZE: u32 = 100;
    pub const ERROR_MSG: &'static str =
        "requested transfer failed. this can be the case if the contract does not\
    have sufficient free funds or if the transfer would have brought the\
    contract's balance below minimum balance.";

    #[ink(storage)]
    #[derive(SpreadAllocate, Storage)]
    pub struct Mixer {
        deposit_size: Balance,
        merkle_tree: merkle_tree::MerkleTree,
        used_nullifiers: Mapping<[u8; 32], bool>,
        poseidon: PoseidonRef,
        verifier: MixerVerifierRef,
        #[storage_field]
        psp22: psp22::Data,
        psp22_token_address: Option<AccountId>,
    }

    impl PSP22 for Mixer {}

    #[ink(event)]
    pub struct Deposit {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        commitment: [u8; 32],
        value: Balance,
    }

    #[ink(event)]
    pub struct Withdraw {
        #[ink(topic)]
        recipient: AccountId,
        #[ink(topic)]
        relayer: AccountId,
        fee: Balance,
        refund: Balance,
    }

    /// The mixer error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if a mapping item is not found
        ItemNotFound,
        /// Returned if the merkle tree is full.
        MerkleTreeIsFull,
        /// Hash error
        HashError,
        /// Verify error
        VerifyError,
        /// Nullifier is known
        NullifierKnown,
        /// Invalid Withdraw Proof
        InvalidWithdrawProof,
        /// TransferError
        TransferError,
        /// PSP22 allowance error
        PSP22AllowanceError,
    }

    /// The mixer result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct WithdrawParams {
        proof_bytes: Vec<u8>,
        root: [u8; 32],
        nullifier_hash: [u8; 32],
        recipient: AccountId,
        relayer: AccountId,
        fee: Balance,
        refund: Balance,
    }

    impl Mixer {
        #[ink(constructor)]
        pub fn new(
            levels: u32,
            deposit_size: Balance,
            version: u32,
            psp22_contract_address: AccountId,
            poseidon_contract_hash: Hash,
            verifier_contract_hash: Hash,
        ) -> Self {
            let salt = version.to_le_bytes();
            let poseidon = PoseidonRef::new()
                .endowment(0)
                .code_hash(poseidon_contract_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    ink_env::debug_print!("contract error in poseidon init {:?}", error);
                    panic!("failed at instantiating the Poseidon contract: {:?}", error)
                });
            let verifier = MixerVerifierRef::new()
                .endowment(0)
                .code_hash(verifier_contract_hash)
                .salt_bytes(salt)
                .instantiate()
                .unwrap_or_else(|error| {
                    panic!(
                        "failed at instantiating the MixerVerifier contract: {:?}",
                        error
                    )
                });

            ink_lang::utils::initialize_contract(|contract: &mut Mixer| {
                contract.deposit_size = deposit_size;
                contract.poseidon = poseidon;
                contract.verifier = verifier;
                contract.merkle_tree.levels = levels;
                contract.merkle_tree.current_root_index = 0;
                contract.merkle_tree.next_index = 0;
                contract.psp22_token_address = Some(psp22_contract_address);

                for i in 0..levels {
                    contract.merkle_tree.filled_subtrees.insert(i, &zeroes(i));
                }

                contract.merkle_tree.roots.insert(0, &zeroes(levels));
            })
        }

        /// Returns the `levels` value.
        #[ink(message)]
        pub fn levels(&self) -> u32 {
            self.merkle_tree.levels
        }

        /// Returns the `deposit_size` value.
        #[ink(message)]
        pub fn deposit_size(&self) -> Balance {
            self.deposit_size
        }

        #[ink(message, payable)]
        pub fn deposit_native(&mut self, commitment: [u8; 32]) -> Result<u32> {
            assert!(
                self.env().transferred_value() == self.deposit_size,
                "Deposit size is not correct"
            );

            let index = self.merkle_tree.insert(self.poseidon.clone(), commitment);

            self.env().emit_event(Deposit {
                from: self.env().caller(),
                commitment,
                value: self.env().transferred_value(),
            });

            index
        }

        #[ink(message)]
        pub fn deposit_psp22(&mut self, commitment: [u8; 32], amount: Balance) -> Result<u32> {
            if self.psp22_token_address.is_none() {
                panic!("psp22 token address is not set");
            }

            assert!(amount == self.deposit_size, "Deposit size is not correct");

            let index = self.merkle_tree.insert(self.poseidon.clone(), commitment);

            if self
                .transfer(self.psp22_token_address.unwrap(), amount, Vec::<u8>::new())
                .is_err()
            {
                return Err(Error::TransferError);
            }

            self.env().emit_event(Deposit {
                from: self.env().caller(),
                commitment,
                value: amount,
            });

            index
        }

        #[ink(message, payable)]
        pub fn send_fund_to_contract(&self) {
            ink_env::debug_println!("funds sent");
        }

        #[ink(message)]
        pub fn withdraw(&mut self, withdraw_params: WithdrawParams) -> Result<()> {
            assert!(
                self.merkle_tree.is_known_root(withdraw_params.root),
                "Root is not known"
            );

            if self.is_known_nullifier(withdraw_params.nullifier_hash) {
                return Err(Error::NullifierKnown);
            }

            let element_encoder = |v: &[u8]| {
                let mut output = [0u8; 32];
                output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
                output
            };
            // Format the public input bytes
            let recipient_bytes = truncate_and_pad(withdraw_params.recipient.as_ref());
            let relayer_bytes = truncate_and_pad(withdraw_params.relayer.as_ref());

            let fee_bytes = &withdraw_params.fee.encode();
            let refund_bytes = &withdraw_params.refund.encode();

            let mut arbitrary_data_bytes = Vec::new();
            arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
            arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
            arbitrary_data_bytes.extend_from_slice(&fee_bytes);
            arbitrary_data_bytes.extend_from_slice(&refund_bytes);
            let arbitrary_input =
                Keccak256::hash(&arbitrary_data_bytes).map_err(|_| Error::HashError)?;

            // Join the public input bytes
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&withdraw_params.nullifier_hash);
            bytes.extend_from_slice(&withdraw_params.root);
            bytes.extend_from_slice(&arbitrary_input);

            // Verify the proof
            let result = self.verify(bytes, withdraw_params.proof_bytes)?;
            if !result {
                return Err(Error::InvalidWithdrawProof);
            }
            // Set used nullifier to true after successfuly verification
            self.used_nullifiers
                .insert(withdraw_params.nullifier_hash, &true);

            let actual_amount = self.deposit_size - withdraw_params.fee;

            // Handle transfers for native and PSP22 tokens
            if !is_account_id_zero(self.psp22_token_address.unwrap()) {
                if self
                    .transfer_from(
                        self.psp22_token_address.unwrap(),
                        withdraw_params.recipient,
                        actual_amount,
                        Vec::<u8>::new(),
                    )
                    .is_err()
                {
                    panic!("{}", ERROR_MSG);
                }
                if self
                    .transfer_from(
                        self.psp22_token_address.unwrap(),
                        withdraw_params.relayer,
                        withdraw_params.fee,
                        Vec::<u8>::new(),
                    )
                    .is_err()
                {
                    panic!("{}", ERROR_MSG);
                }
            } else {
                if self
                    .env()
                    .transfer(withdraw_params.recipient, actual_amount)
                    .is_err()
                {
                    panic!("{}", ERROR_MSG);
                }

                if self
                    .env()
                    .transfer(withdraw_params.relayer, withdraw_params.fee)
                    .is_err()
                {
                    panic!("{}", ERROR_MSG);
                }

                if withdraw_params.refund > 0 {
                    if self
                        .env()
                        .transfer(withdraw_params.recipient, withdraw_params.refund)
                        .is_err()
                    {
                        panic!("{}", ERROR_MSG);
                    }
                }

                self.env().emit_event(Withdraw {
                    recipient: withdraw_params.recipient,
                    relayer: withdraw_params.relayer,
                    fee: withdraw_params.fee,
                    refund: withdraw_params.refund,
                });
            }

            Ok(())
        }

        #[ink(message)]
        pub fn insert_nullifier(&mut self, nullifier: [u8; 32]) {
            self.used_nullifiers.insert(&nullifier, &true);
        }

        /// Returns native contract address
        #[ink(message)]
        pub fn native_contract_account_id(&self) -> Option<AccountId> {
            Some(self.env().account_id())
        }

        fn verify(&self, public_input: Vec<u8>, proof_bytes: Vec<u8>) -> Result<bool> {
            self.verifier
                .verify(public_input, proof_bytes)
                .map_err(|_| Error::VerifyError)
        }

        #[ink(message)]
        pub fn is_known_nullifier(&self, nullifier: [u8; 32]) -> bool {
            self.used_nullifiers.get(&nullifier).unwrap_or(false)
        }

        /// Returns psp22 contract balance
        #[ink(message)]
        pub fn native_contract_balance(&self) -> Balance {
            self.env().balance()
        }

        /// Returns psp22 contract balance
        #[ink(message)]
        pub fn psp22_contract_balance(&self) -> Balance {
            self.balance_of(self.psp22_token_address.unwrap())
        }

        /// sets the psp22 allowance for the spender(spend on behalf of owner)
        ///
        /// * `owner` - owner's address
        /// * `spender` - spender's address
        /// * `amount` - amount to spend
        #[ink(message)]
        pub fn set_psp22_allowance_for_owner(
            &mut self,
            owner: AccountId,
            spender: AccountId,
            amount: Balance,
        ) -> Result<()> {
            // psp22 call to increase allowance
            self.psp22.allowances.insert(&(&owner, &spender), &amount);
            Ok(())
        }

        /// Gets the psp22 allowance for the spender(spend on behalf of owner)
        ///
        /// * `owner` - owner's address
        /// * `spender` - spender's address
        #[ink(message)]
        pub fn get_psp22_allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance(owner, spender)
        }

        /// Insert's psp22 token for an address
        ///
        /// * `account_id` - address to transfer to
        /// * `amount` - amount to transfer
        #[ink(message)]
        pub fn insert_psp22_balance(
            &mut self,
            account_id: AccountId,
            amount: Balance,
        ) -> Result<()> {
            self.psp22.balances.insert(&account_id, &amount);
            Ok(())
        }

        /// Returns psp22 balance for an address
        ///
        /// * `address` - The address to check
        #[ink(message)]
        pub fn psp22_balance(&self, address: AccountId) -> Balance {
            self.balance_of(address)
        }
    }
}
