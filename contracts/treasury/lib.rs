#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

pub use crate::treasury::{Treasury, TreasuryRef};
use ink_env::call::FromAccountId;
use ink_lang as ink;
use ink_storage::traits::SpreadAllocate;

impl SpreadAllocate for TreasuryRef {
    fn allocate_spread(_ptr: &mut ink_primitives::KeyPtr) -> Self {
        FromAccountId::from_account_id([0; 32].into())
    }
}
#[ink::contract]
mod treasury {
    use brush::contracts::psp22::*;
    use brush::contracts::traits::psp22::PSP22;
    use ink_prelude::vec::Vec;
    use ink_storage::{traits::SpreadAllocate, Mapping};
    use protocol_ink_lib::utils::{is_account_id_zero, ZERO_ADDRESS};

    /// The treasury result type.
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    #[derive(SpreadAllocate, PSP22Storage)]
    pub struct Treasury {
        #[PSP22StorageField]
        psp22: PSP22Data,
        treasury_handler: AccountId,
        proposal_nonce: u32,
    }

    /// The treasury error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Unauthorized
        Unauthorized,
        /// Invalid Nonce
        InvalidNonce,
        /// Invalid Rescue Amount
        InvalidRescueAmount,
        /// Invalid Address
        InvalidAddress,
        /// Transfer Error
        TransferError,
    }

    impl Treasury {
        #[ink(constructor)]
        pub fn new(treasury_handler: AccountId) -> Self {
            ink_lang::codegen::initialize_contract(|instance: &mut Self| {
                instance.treasury_handler = treasury_handler;
                instance.proposal_nonce = 0;
            })
        }

        /// Rescues tokens
        ///
        /// * `token_address` -  The token address
        /// * `to` -  Location address to rescue the token to
        /// * `amount_to_rescue` -  The amount to rescue
        /// * `nonce` - The nonce to use
        #[ink(message, payable)]
        pub fn rescue_tokens(
            &mut self,
            token_address: AccountId,
            to: AccountId,
            amount_to_rescue: Balance,
            nonce: u32,
        ) -> Result<()> {
            if is_account_id_zero(to) {
                return Err(Error::InvalidAddress);
            }

            if self.env().caller() != self.treasury_handler {
                return Err(Error::Unauthorized);
            }

            if self.proposal_nonce > nonce || self.proposal_nonce + 1048 < nonce {
                return Err(Error::InvalidNonce);
            }

            if amount_to_rescue == 0 {
                return Err(Error::InvalidRescueAmount);
            }

            if is_account_id_zero(token_address) {
                let native_balance = self.env().balance();

                if native_balance >= amount_to_rescue {
                    if self.env().transfer(to, amount_to_rescue).is_err() {
                        return Err(Error::TransferError);
                    }
                } else {
                    if self.env().transfer(to, native_balance).is_err() {
                        return Err(Error::TransferError);
                    }
                }
            } else {
                let psp22_balance = self.balance_of(self.env().account_id());

                if psp22_balance >= amount_to_rescue {
                    if self
                        .transfer_from(token_address, to, amount_to_rescue, Vec::<u8>::new())
                        .is_err()
                    {
                        return Err(Error::TransferError);
                    }
                } else {
                    if self
                        .transfer_from(token_address, to, psp22_balance, Vec::<u8>::new())
                        .is_err()
                    {
                        return Err(Error::TransferError);
                    }
                }
            }

            self.proposal_nonce = nonce;
            Ok(())
        }

        /// Used to set new treasury handler address
        ///
        /// * `handler` -  The new address to be set
        /// * `nonce` - The nonce to use
        #[ink(message)]
        pub fn set_handler(&mut self, handler: AccountId, nonce: u32) -> Result<()> {
            if self.env().caller() != self.treasury_handler {
                return Err(Error::Unauthorized);
            }

            if self.proposal_nonce > nonce || self.proposal_nonce + 1048 < nonce {
                return Err(Error::InvalidNonce);
            }

            self.proposal_nonce = nonce;
            self.treasury_handler = handler;

            Ok(())
        }

        /// Returns the `handler` value.
        #[ink(message)]
        pub fn handler(&self) -> AccountId {
            self.treasury_handler
        }

        /// Returns the `proposal_nonce` value.
        #[ink(message)]
        pub fn nonce(&self) -> u32 {
            self.proposal_nonce
        }

        /// Returns native contract balance
        #[ink(message)]
        pub fn native_contract_balance(&self) -> Balance {
            self.env().balance()
        }

        /// Returns contract psp22 balance
        #[ink(message)]
        pub fn psp22_contract_balance(&self) -> Balance {
            self.balance_of(self.env().account_id())
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
            self.psp22.allowances.insert((owner, spender), &amount);
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

        /// Returns psp22 balance for an address
        ///
        /// * `address` - The address to check
        #[ink(message)]
        pub fn psp22_balance(&self, address: AccountId) -> Balance {
            self.balance_of(address)
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
    }
}
