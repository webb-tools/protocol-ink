#![cfg_attr(not(feature = "std"), no_std)]

mod signing;

use ink_lang as ink;

#[ink::contract]
mod signature_bridge {
    use super::signing::SignatureVerifier;
    use crate::signing::SigningSystem;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::traits::{PackedLayout, SpreadLayout, StorageLayout};
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use webb_proposals::TypedChainId;

    /// The vanchor result type.
    pub type Result<T> = core::result::Result<T, Error>;

    /// The token wrapper error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Invalid Signature From Governor
        InvalidSignatureFromGovernor,
        /// Invalid Nonce
        InvalidNonce,
        /// Invalid Function Sig
        InvalidFunctionSig,
        /// Executing on wrong chain
        WrongChainExecution,
        /// Invalid Resource Id
        InvalidResourceId,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct SignatureBridge {
        governor: Vec<u8>,
        proposal_nonce: u32,
        chain_id: u64,
        counts: Mapping<Vec<u8>, [u8; 32]>,
        resource_id_to_handler_address: Mapping<[u8; 32], AccountId>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ResourceParams {
        resource_id: [u8; 32],
        function_sig: [u8; 4],
        nonce: u32,
        new_resource_id: [u8; 32],
        handler_address: AccountId,
        execution_context_address: AccountId,
        sig: Vec<u8>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ExecuteProposalParams {
        data: Vec<u8>,
        sig: Vec<u8>,
    }

    impl SignatureBridge {
        #[ink(constructor)]
        pub fn new(governor: Vec<u8>, proposal_nonce: u32, chain_id: u64) -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut Self| {
                instance.governor = governor;
                instance.proposal_nonce = proposal_nonce;
                instance.chain_id = chain_id;
            })
        }

        #[ink(message)]
        pub fn admin_set_resource_with_signature(
            &mut self,
            resource_params: ResourceParams,
        ) -> Result<()> {
            // Validations
            let mut data: Vec<u8> = Vec::new();
            data.extend_from_slice(&resource_params.resource_id);
            data.extend_from_slice(&resource_params.function_sig);
            data.extend_from_slice(&resource_params.nonce.to_be_bytes());
            data.extend_from_slice(&resource_params.new_resource_id);
            let handler_address_bytes = truncate_and_pad(resource_params.handler_address.as_ref());
            data.extend_from_slice(&*handler_address_bytes);
            let execution_context_address_bytes =
                truncate_and_pad(resource_params.execution_context_address.as_ref());
            data.extend_from_slice(&*execution_context_address_bytes);

            if !self.is_signed_by_governor(&data, &resource_params.sig) {
                return Err(Error::InvalidSignatureFromGovernor);
            }

            if resource_params.nonce <= self.proposal_nonce
                || self.proposal_nonce + 1048 < resource_params.nonce
            {
                return Err(Error::InvalidNonce);
            }

            if resource_params.function_sig != [0u8; 4] {
                return Err(Error::InvalidFunctionSig);
            }

            // Save the info of "resource_id -> handler(contract)" in this contract.
            self.resource_id_to_handler_address.insert(
                resource_params.new_resource_id,
                &resource_params.handler_address,
            );

            self.proposal_nonce = resource_params.nonce;

            // TODO: Execute call to specific handler contract such as Token Wrapper Handler, Anchor Handler, Treasury Handler e.t.c

            Ok(())
        }

        pub fn execute_proposal_with_signature(
            &mut self,
            data: Vec<u8>,
            sig: Vec<u8>,
        ) -> Result<()> {
            if !self.is_signed_by_governor(&data, &sig) {
                return Err(Error::InvalidSignatureFromGovernor);
            }

            // Parse resourceID from the data
            let resource_id_bytes = &data[0..32];
            let resource_id = element_encoder(resource_id_bytes);

            // Parse chain ID + chain type from the resource ID
            let typed_chain_id_bytes: [u8; 6] = resource_id_bytes[26..32].try_into().unwrap();
            let mut padded_typed_chain_id_bytes: [u8; 8] = [0; 8];
            padded_typed_chain_id_bytes[..typed_chain_id_bytes.len()]
                .copy_from_slice(&typed_chain_id_bytes);

            let execution_typed_chain =
                TypedChainId::from(u64::from_be_bytes(padded_typed_chain_id_bytes));
            let execution_chain_id_type: u64 = execution_typed_chain.chain_id();

            if TypedChainId::Ink(self.chain_id as u32).chain_id() != execution_chain_id_type {
                return Err(Error::WrongChainExecution);
            }

            let handler_address = self.resource_id_to_handler_address.get(resource_id);

            if handler_address.is_none() {
                return Err(Error::InvalidResourceId);
            }

            //TODO: Execute the "proposal" in "handler" contract such as Token Wrapper Handler, Anchor Handler, Treasury Handler e.t.c

            Ok(())
        }

        fn is_signed_by_governor(&mut self, data: &[u8], sig: &[u8]) -> bool {
            let result = SignatureVerifier::verify(&self.governor, data, sig)
                .unwrap_or_else(|error| panic!("could not verify due to: {:?}", error));
            result
        }
    }

    pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
        let mut truncated_bytes = t[..20].to_vec();
        truncated_bytes.extend_from_slice(&[0u8; 12]);
        truncated_bytes
    }

    pub fn element_encoder(v: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
        output
    }
}
