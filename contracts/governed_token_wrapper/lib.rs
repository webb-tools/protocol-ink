#![cfg_attr(not(feature = "std"), no_std)]
#![feature(min_specialization)]

use ink_lang as ink;

#[brush::contract]
mod governed_token_wrapper {
    use brush::contracts::psp22::extensions::burnable::*;
    use brush::contracts::psp22::extensions::metadata::*;
    use brush::contracts::psp22::extensions::mintable::*;
    use brush::contracts::psp22::extensions::wrapper::*;
    use brush::contracts::psp22::*;
    use brush::contracts::traits::psp22::PSP22;
    use brush::test_utils::*;
    use ink_prelude::string::String;
    use ink_prelude::vec::Vec;
    use ink_storage::traits::{PackedLayout, SpreadLayout, StorageLayout};
    use ink_storage::{traits::SpreadAllocate, Mapping};

    /// The vanchor result type.
    pub type Result<T> = core::result::Result<T, Error>;
    pub const ERROR_MSG: &'static str =
        "requested transfer failed. this can be the case if the contract does not\
    have sufficient free funds or if the transfer would have brought the\
    contract's balance below minimum balance.";

    #[ink(storage)]
    #[derive(Default, SpreadAllocate, PSP22Storage, PSP22WrapperStorage, PSP22MetadataStorage)]
    pub struct GovernedTokenWrapper {
        #[PSP22StorageField]
        psp22: PSP22Data,
        #[PSP22MetadataStorageField]
        metadata: PSP22MetadataData,
        #[PSP22WrapperStorageField]
        wrapper: PSP22WrapperData,

        // /// Governance - related params
        governor: AccountId,
        fee_recipient: AccountId,
        fee_percentage: Balance,
        is_native_allowed: bool,
        wrapping_limit: u128,
        proposal_nonce: u64,

        tokens: Mapping<AccountId, bool>,
        historical_tokens: Mapping<AccountId, bool>,

        valid: Mapping<AccountId, bool>,
        historically_valid: Mapping<AccountId, bool>,
    }

    impl PSP22 for GovernedTokenWrapper {}

    impl PSP22Metadata for GovernedTokenWrapper {}

    impl PSP22Mintable for GovernedTokenWrapper {}

    impl PSP22Wrapper for GovernedTokenWrapper {}

    impl PSP22Burnable for GovernedTokenWrapper {}

    /// The token wrapper error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Invalid amount provided for native wrapping
        InvalidAmountForNativeWrapping,
        /// Native wrapping is not allowed for this token wrapper
        NativeWrappingNotAllowed,
        /// Invalid value sent for wrapping
        InvalidValueSentForWrapping,
        /// Invalid token address
        InvalidTokenAddress,
        /// Token Address already exists
        TokenAddressAlreadyExists,
        /// Invalid token amount
        InvalidTokenAmount,
        /// Insufficient native balance
        InsufficientNativeBalance,
        /// Native unwrapping is not allowed for this token wrapper
        NativeUnwrappingNotAllowed,
        /// Insufficient PSP22 balance
        InsufficientPSP22Balance,
        /// Invalid historical token address
        InvalidHistoricalTokenAddress,
        /// Unauthorized
        Unauthorize,
        /// Invalid Nonce
        InvalidNonce,
        /// Nonce must increment by 1
        NonceMustIncrementByOne,
        /// TransferError
        TransferError
    }

    #[ink(event)]
    pub struct Wrap {
        #[ink(topic)]
        sender: Option<AccountId>,
        #[ink(topic)]
        mint_for: Option<AccountId>,
        amount: Balance,
    }

    impl GovernedTokenWrapper {
        #[ink(constructor)]
        pub fn new(
            name: Option<String>,
            symbol: Option<String>,
            decimal: u8,
            governor: AccountId,
            fee_recipient: AccountId,
            fee_percentage: Balance,
            is_native_allowed: bool,
            wrapping_limit: u128,
            proposal_nonce: u64,
            token_address: AccountId,
            total_supply: Balance,
            governor_balance: Balance
        ) -> Self {
            ink_env::debug_println!("created new instance of token wrapper at {}", Self::env().block_number());

            ink_lang::codegen::initialize_contract(|instance: &mut Self| {
                // for wrapping
                instance._init(token_address);

                instance.metadata.name = name;
                instance.metadata.symbol = symbol;
                instance.metadata.decimals = decimal;

                instance.psp22.supply = total_supply;
                instance.psp22.balances.insert(&governor, &governor_balance);

                // Governance config
                instance.governor = governor;
                instance.fee_recipient = fee_recipient;
                instance.fee_percentage = fee_percentage;
                instance.is_native_allowed = is_native_allowed;
                instance.wrapping_limit = wrapping_limit;
                instance.proposal_nonce = proposal_nonce;
            })
        }

        /// Used to wrap tokens on behalf of a sender.
        ///
        /// token_address is the address of PSP22 to transfer to, if token_address is None,
        /// then it's a Native token address
        ///
        /// amount is the amount of token to transfer
        #[ink(message, payable)]
        pub fn wrap(&mut self, token_address: Option<AccountId>, amount: Balance) -> Result<()> {
            ink_env::debug_println!("inside wrap");

            self.is_valid_wrapping(token_address, amount);

            // determine amount to use
            let amount_to_use = if token_address.is_none() {
                self.env().transferred_value()
            } else {
                amount
            };

            //let cost_to_wrap = self.get_fee_from_amount(amount_to_use);
            let cost_to_wrap = 10;

            let message = ink_prelude::format!("cost_to_wrap is {:?}", cost_to_wrap);
            ink_env::debug_println!("{}", &message);

            //let leftover = amount_to_use.saturating_sub(cost_to_wrap);
            let leftover = 4;

            let message = ink_prelude::format!("leftover is {:?}", leftover);
            ink_env::debug_println!("{}", &message);

            self.do_wrap(
                token_address.clone(),
                self.env().caller(),
                self.env().caller(),
                cost_to_wrap,
                leftover,
            );

            Ok(())
        }

        /// Used to unwrap/burn the wrapper token on behalf of a sender.
        ///
        /// token_address is the address of PSP22 to transfer to, if token_address is None,
        /// then it's a Native token address
        ///
        /// amount is the amount of token to transfer
        #[ink(message, payable)]
        pub fn unwrap(&mut self, token_address: Option<AccountId>, amount: Balance) {
            self.is_valid_unwrapping(token_address, amount);

            self.do_unwrap(
                token_address.clone(),
                self.env().caller(),
                self.env().caller(),
                amount,
            );
        }

        /// Used to unwrap/burn the wrapper token on behalf of a sender.
        ///
        /// token_address is the address of PSP22 to unwrap into,
        ///
        /// amount is the amount of tokens to burn
        ///
        /// recipient is the address to transfer to
        #[ink(message, payable)]
        pub fn unwrap_and_send_to(
            &mut self,
            token_address: Option<AccountId>,
            amount: Balance,
            recipient: AccountId,
        ) {
            self.is_valid_unwrapping(token_address, amount);

            self.do_unwrap(
                token_address.clone(),
                recipient,
                self.env().caller(),
                amount,
            );
        }

        /// Used to wrap tokens on behalf of a sender
        ///
        /// token_address is the address of PSP22 to unwrap into,
        ///
        /// amount is the amount of tokens to transfer
        ///
        /// sender is the Address of sender where assets are sent from.
        #[ink(message, payable)]
        pub fn wrap_for(
            &mut self,
            token_address: Option<AccountId>,
            sender: AccountId,
            amount: Balance,
        ) {
            self.is_valid_wrapping(token_address, amount);

            // determine amount to use
            let amount_to_use = if token_address.is_none() {
                self.env().transferred_value()
            } else {
                amount
            };

            let cost_to_wrap = self.get_fee_from_amount(amount_to_use);

            let leftover = amount_to_use.saturating_sub(cost_to_wrap);

            self.do_wrap(
                token_address.clone(),
                sender,
                sender,
                cost_to_wrap,
                leftover,
            );

            self.env().emit_event(Wrap {
                sender: Some(sender),
                mint_for: Some(sender),
                amount,
            });
        }
        /// Used to wrap tokens on behalf of a sender and mint to a potentially different address
        ///
        /// token_address is the address of PSP22 to unwrap into,
        ///
        /// sender is Address of sender where assets are sent from.
        ///
        /// amount is the amount of tokens to transfer
        ///
        /// Recipient is the recipient of the wrapped tokens.
        #[ink(message, payable)]
        pub fn wrap_for_and_send_to(
            &mut self,
            token_address: Option<AccountId>,
            sender: AccountId,
            amount: Balance,
            recipient: AccountId,
        ) {
            self.is_valid_wrapping(token_address, amount);

            // determine amount to use
            let amount_to_use = if token_address.is_none() {
                self.env().transferred_value()
            } else {
                amount
            };

            let cost_to_wrap = self.get_fee_from_amount(amount_to_use);

            let leftover = amount_to_use.saturating_sub(cost_to_wrap);

            self.do_wrap(
                token_address.clone(),
                sender,
                recipient,
                cost_to_wrap,
                leftover,
            );
        }

        /// Used to unwrap/burn the wrapper token on behalf of a sender.
        ///
        /// token_address is the address of PSP22 to transfer to, if token_address is None,
        /// then it's a Native token address
        ///
        /// amount is the amount of token to transfer
        ///
        /// sender is the Address of sender where liquidity is send to.
        #[ink(message, payable)]
        pub fn unwrap_for(
            &mut self,
            token_address: Option<AccountId>,
            amount: Balance,
            sender: AccountId,
        ) {
            self.is_valid_unwrapping(token_address, amount);
            self.do_unwrap(token_address.clone(), sender, sender, amount);
        }

        ///  Adds a token at `_tokenAddress` to the GovernedTokenWrapper's wrapping list
        ///
        /// tokenAddress:  The address of the token to be added
        ///
        /// nonce: The nonce tracking updates to this contract
        #[ink(message)]
        pub fn add_token_address(&mut self, token_address: AccountId, nonce: u64) -> Result<()> {
            ink_env::debug_println!("inside add token address");

            let message = ink_prelude::format!("caller is {:?}", self.env().caller());
            ink_env::debug_println!("{}", &message);

            let message = ink_prelude::format!("governor is {:?}", self.governor);
            ink_env::debug_println!("{}", &message);

            // only contract governor can execute this function
            self.is_governor(self.env().caller());


            // check if token address already exists
            if self.is_valid_address(token_address) {
                ink_env::debug_println!("token address already exists");
                return Err(Error::TokenAddressAlreadyExists);
            }

            if self.proposal_nonce > nonce {
                ink_env::debug_println!("invalid nonce");
                return Err(Error::InvalidNonce);
            }

            if nonce != self.proposal_nonce + 1 {
                ink_env::debug_println!("nonce must increment by one");
                return Err(Error::NonceMustIncrementByOne);
            }

            self.valid.insert(token_address, &true);
            self.historically_valid.insert(token_address, &true);
            self.tokens.insert(token_address, &true);
            self.historical_tokens.insert(token_address, &true);

            self.proposal_nonce = nonce;

            ink_env::debug_println!("done literally");

            Ok(())
        }

        ///  Removes a token at `_tokenAddress` from the GovernedTokenWrapper's wrapping list
        ///
        /// tokenAddress:  The address of the token to be added
        ///
        /// nonce: The nonce tracking updates to this contract
        #[ink(message)]
        pub fn remove_token_address(&mut self, token_address: AccountId, nonce: u64) -> Result<()> {
            ink_env::debug_println!("inside remove token address");
            self.is_governor(self.env().caller());

            // check if token address already exists
            if !self.is_valid_address(token_address) {
                ink_env::debug_println!("invalid token address");
                return Err(Error::InvalidTokenAddress);
            }

            if self.proposal_nonce > nonce {
                ink_env::debug_println!("invalid nonce");
                return Err(Error::InvalidNonce);
            }

            if nonce != self.proposal_nonce + 1 {
                ink_env::debug_println!("nonce must increment by 1");
                return Err(Error::NonceMustIncrementByOne);
            }

            self.valid.insert(token_address, &false);
            self.tokens.insert(token_address, &false);

            self.proposal_nonce = nonce;
            Ok(())
        }

        /// Updates contract configs
        #[ink(message)]
        pub fn update_config(
            &mut self,
            governor: Option<AccountId>,
            is_native_allowed: Option<bool>,
            wrapping_limit: Option<u128>,
            fee_percentage: Option<Balance>,
            fee_recipient: Option<AccountId>,
        ) {
            // only contract governor can execute this function
            self.is_governor(self.env().caller());

            if governor.is_some() {
                self.governor = governor.unwrap();
            }

            if is_native_allowed.is_some() {
                self.is_native_allowed = is_native_allowed.unwrap();
            }

            if wrapping_limit.is_some() {
                self.wrapping_limit = wrapping_limit.unwrap_or(self.wrapping_limit);
            }

            if fee_percentage.is_some() {
                self.fee_percentage = fee_percentage.unwrap();
            }

            if fee_recipient.is_some() {
                self.fee_recipient = fee_recipient.unwrap();
            }
        }

        /// Handles unwrapping by transferring token to the sender and burning for the burn_for address
        fn do_unwrap(
            &mut self,
            token_address: Option<AccountId>,
            sender: AccountId,
            burn_for: AccountId,
            amount: Balance,
        ) {
            // burn wrapped token from sender
            self.burn(burn_for, amount);

            if token_address.is_none() {
                // transfer native liquidity from the token wrapper to the sender
                if self.env().transfer(sender, amount).is_err() {
                    panic!("{}", ERROR_MSG);
                }
            } else {
                // transfer PSP22 liquidity from the token wrapper to the sender
                self.transfer(sender, amount, Vec::<u8>::new()).is_ok();
            }
        }

        /// Handles wrapping by transferring token to the sender and minting for the mint_for address
        fn do_wrap(
            &mut self,
            token_address: Option<AccountId>,
            sender: AccountId,
            mint_for: AccountId,
            cost_to_wrap: Balance,
            leftover: Balance,
        ) -> Result<()> {
            ink_env::debug_println!("doing wrap");

            if token_address.is_none() {
                ink_env::debug_println!("native token transfer");
                // mint the native value sent to the contract
                self.mint(mint_for, leftover);

                // transfer costToWrap to the feeRecipient
                if self
                    .env()
                    .transfer(self.fee_recipient, cost_to_wrap)
                    .is_err()
                {
                    return Err(Error::TransferError);
                    panic!("{}", ERROR_MSG);
                }
            } else {
                ink_env::debug_println!("psp22 token transfer");
                // psp22 transfer of liquidity to token wrapper contract
                self.transfer_from(sender, self.env().account_id(), leftover, Vec::<u8>::new())
                    .is_ok();

                // psp22 transfer to fee recipient
                self.transfer_from(sender, self.fee_recipient, cost_to_wrap, Vec::<u8>::new())
                    .is_ok();

                // mint the wrapped token for the sender
                self.mint(mint_for, leftover);
            }

            Ok(())
        }

        /// Checks to determine if it's safe to wrap
        fn is_valid_wrapping(
            &mut self,
            token_address: Option<AccountId>,
            amount: Balance,
        ) -> Result<()> {
            if token_address.is_none() {
                if amount != 0 {
                    return Err(Error::InvalidAmountForNativeWrapping);
                }

                if !self.is_native_allowed {
                    return Err(Error::NativeWrappingNotAllowed);
                }
            } else {
                if self.env().transferred_value() != 0 {
                    ink_env::debug_println!("value is not equal to 0");
                    return Err(Error::InvalidValueSentForWrapping);
                }

                if !self.is_valid_address(token_address.unwrap()) {
                    ink_env::debug_println!("invalid token address");
                    return Err(Error::InvalidTokenAddress);
                }
            }

            if !self.is_valid_amount(amount) {
                let message = ink_prelude::format!("amount to wrap is {:?}", amount);
                ink_env::debug_println!("{}", &message);

                ink_env::debug_println!("invalid token amount");
                return Err(Error::InvalidTokenAmount);
            }

            ink_env::debug_println!("wrapping is valid");

            Ok(())
        }

        fn is_valid_unwrapping(
            &mut self,
            token_address: Option<AccountId>,
            amount: Balance,
        ) -> Result<()> {
            if token_address.is_none() {
                if amount >= self.env().balance() {
                    return Err(Error::InsufficientNativeBalance);
                }

                if !self.is_native_allowed {
                    return Err(Error::NativeUnwrappingNotAllowed);
                }
            } else {
                if amount >= self.balance_of(self.env().account_id()) {
                    return Err(Error::InsufficientPSP22Balance);
                }

                if !self.is_address_historically_valid(token_address.unwrap()) {
                    return Err(Error::InvalidHistoricalTokenAddress);
                }
            }

            Ok(())
        }

        /// Determines if token address is a valid one
        fn is_valid_address(&mut self, token_address: AccountId) -> bool {
           let res =  self.valid.get(token_address).is_some();
            let message = ink_prelude::format!("res is {:?}", res);
            ink_env::debug_println!("{}", &message);

            let message = ink_prelude::format!("nonce is {:?}", self.proposal_nonce);
            ink_env::debug_println!("{}", &message);
           res
        }

        /// Determines if token address is historically valid
        fn is_address_historically_valid(&mut self, token_address: AccountId) -> bool {
            self.historically_valid.get(token_address).is_some()
        }

        /// Determines if amount is valid for wrapping
        fn is_valid_amount(&mut self, amount: Balance) -> bool {
            let amount_add_supply =  amount.saturating_add(self.psp22.supply);
            let message = ink_prelude::format!("addition of amount to supply is {:?}", amount);
            ink_env::debug_println!("{}", &message);

            let message = ink_prelude::format!("wrapping limit is{:?}", self.wrapping_limit);
            ink_env::debug_println!("{}", &message);

            amount_add_supply <= self.wrapping_limit
        }

        /// Calculates the fee to be sent to fee recipient
        fn get_fee_from_amount(&mut self, amount_to_wrap: Balance) -> Balance {
            amount_to_wrap
                .saturating_mul(self.fee_percentage)
                .saturating_div(100)
        }

        fn is_governor(&mut self, address: AccountId) -> Result<()> {
            if self.governor != address {
                return Err(Error::Unauthorize);
            }

            Ok(())
        }

        /// Returns the `governor` value.
        #[ink(message)]
        pub fn governor(&self) -> AccountId {
            self.governor
        }

        #[ink(message)]
        pub fn name(&self) -> Option<String> {
            self.metadata.name.clone()
        }

        #[ink(message)]
        pub fn nonce(&self) -> u64 {
            let message = ink_prelude::format!("nonce in query is {:?}", self.proposal_nonce);
            ink_env::debug_println!("{}", &message);
            self.proposal_nonce
        }

        #[ink(message)]
        pub fn is_valid_token_address(&self, token_address: AccountId) -> bool {
            self.valid.get(token_address).unwrap()
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.psp22.supply
        }
    }
}
