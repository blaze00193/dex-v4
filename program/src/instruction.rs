#![allow(clippy::too_many_arguments)]
use std::mem::size_of;

use bytemuck::{bytes_of, Pod};
use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::processor::INSTRUCTION_TAG_OFFSET;
pub use crate::processor::{
    cancel_order, close_market, consume_events, create_market, initialize_account, new_order,
    settle, sweep_fees,
};
#[derive(Clone, Copy, FromPrimitive, ToPrimitive)]
/// Describes all possible instructions and their required accounts
pub enum DexInstruction {
    /// Creates a new DEX market
    ///
    /// | index | writable | signer | description                                  |
    /// |-------|----------|--------|----------------------------------------------|
    /// | 0     | ✅        | ❌      | The market account                           |
    /// | 1     | ❌        | ❌      | The orderbook account                        |
    /// | 2     | ❌        | ❌      | The base token vault account                 |
    /// | 4     | ❌        | ❌      | The quote token vault account                |
    /// | 5     | ❌        | ❌      | The Asset Agnostic Orderbook program account |
    /// | 6     | ❌        | ❌      | The market admin account                     |
    CreateMarket,
    /// Execute a new order instruction. Supported types include Limit, IOC, FOK, or Post only.
    ///
    /// | index | writable | signer | description                                                                        |
    /// |-------|----------|--------|------------------------------------------------------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program                                               |
    /// | 1     | ❌        | ❌      | The SPL token program                                                              |
    /// | 3     | ❌        | ❌      | The system program                                                                 |
    /// | 4     | ✅        | ❌      | The DEX market                                                                     |
    /// | 5     | ❌        | ❌      | The DEX market signer                                                              |
    /// | 6     | ✅        | ❌      | The orderbook                                                                      |
    /// | 7     | ✅        | ❌      | The event queue                                                                    |
    /// | 8     | ✅        | ❌      | The bids shared memory                                                             |
    /// | 9     | ✅        | ❌      | The asks shared memory                                                             |
    /// | 10    | ✅        | ❌      | The base token vault                                                               |
    /// | 11    | ✅        | ❌      | The quote token vault                                                              |
    /// | 12    | ✅        | ❌      | The DEX user account                                                               |
    /// | 13    | ✅        | ❌      | The user's source token account                                                    |
    /// | 14    | ✅        | ❌      | The user's wallet                                                                  |
    /// | 15    | ✅        | ❌      | The optional SRM or MSRM discount token account (must be owned by the user wallet) |
    NewOrder,
    /// Cancel an existing order and remove it from the orderbook.
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program |
    /// | 1     | ❌        | ❌      | The DEX market                       |
    /// | 2     | ❌        | ❌      | The DEX market signer                |
    /// | 3     | ✅        | ❌      | The orderbook                        |
    /// | 4     | ✅        | ❌      | The event queue                      |
    /// | 5     | ✅        | ❌      | The bids shared memory               |
    /// | 6     | ✅        | ❌      | The asks shared memory               |
    /// | 7     | ✅        | ❌      | The DEX user account                 |
    /// | 8     | ❌        | ✅      | The user's wallet                    |
    CancelOrder,
    /// Crank the processing of DEX events.
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The asset agnostic orderbook program |
    /// | 1     | ❌        | ❌      | The DEX market                       |
    /// | 2     | ❌        | ❌      | The DEX market signer                |
    /// | 3     | ✅        | ❌      | The orderbook                        |
    /// | 4     | ✅        | ❌      | The event queue                      |
    /// | 5     | ✅        | ❌      | The reward target                    |
    /// | 8..   | ✅        | ❌      | The relevant user account            |
    ConsumeEvents,
    /// Extract available base and quote token assets from a user account
    ///
    /// | index | writable | signer | description                          |
    /// |-------|----------|--------|--------------------------------------|
    /// | 0     | ❌        | ❌      | The spl token program                |
    /// | 1     | ❌        | ❌      | The DEX market                       |
    /// | 2     | ✅        | ❌      | The base token vault                 |
    /// | 3     | ✅        | ❌      | The quote token vault                |
    /// | 4     | ❌        | ❌      | The DEX market signer                |
    /// | 5     | ✅        | ❌      | The DEX user account                 |
    /// | 6     | ❌        | ✅      | The DEX user account owner wallet    |
    /// | 7     | ✅        | ❌      | The destination base token account   |
    /// | 8     | ✅        | ❌      | The destination quote token account  |
    Settle,
    /// Initialize a new user account
    ///
    /// | index | writable | signer | description                    |
    /// |-------|----------|--------|--------------------------------|
    /// | 0     | ❌        | ❌      | The system program             |
    /// | 1     | ✅        | ❌      | The user account to initialize |
    /// | 2     | ❌        | ✅      | The owner of the user account  |
    /// | 3     | ✅        | ✅      | The fee payer                  |
    InitializeAccount,
    /// Extract accumulated fees from the market. This is an admin instruction
    ///
    /// | index | writable | signer | description                   |
    /// |-------|----------|--------|-------------------------------|
    /// | 0     | ✅        | ❌      | The DEX market                |
    /// | 1     | ❌        | ❌      | The market signer             |
    /// | 2     | ❌        | ✅      | The market admin              |
    /// | 3     | ✅        | ❌      | The market quote token vault  |
    /// | 4     | ✅        | ❌      | The destination token account |
    /// | 5     | ❌        | ❌      | The SPL token program         |
    SweepFees,
    /// Close an inactive and empty user account
    ///
    /// | index | writable | signer | description                            |
    /// |-------|----------|--------|----------------------------------------|
    /// | 0     | ✅        | ❌      | The user account to close              |
    /// | 1     | ❌        | ✅      | The owner of the user account to close |
    /// | 2     | ✅        | ❌      | The target lamports account            |
    CloseAccount,
    // Close an existing market
    ///
    // | index | writable | signer | description                    |
    // |-------|----------|--------|--------------------------------|
    // | 0     | ✅        | ❌      | The market account             |
    // | 1     | ✅        | ❌      | The market base vault account  |
    // | 2     | ✅        | ❌      | The market quote vault account |
    // | 3     | ✅        | ❌      | The DEX market signer          |
    // | 4     | ✅        | ❌      | The orderbook account          |
    // | 5     | ✅        | ❌      | The event queue account        |
    // | 6     | ✅        | ❌      | The bids account               |
    // | 7     | ✅        | ❌      | The asks account               |
    // | 8     | ❌        | ❌      | The AAOB program account       |
    // | 9     | ❌        | ✅      | The market admin account       |
    // | 10    | ✅        | ❌      | The target lamports account    |
    CloseMarket,
}

impl DexInstruction {
    pub(crate) fn serialize<T: Pod>(&self, params: T) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::with_capacity(size_of::<T>() + INSTRUCTION_TAG_OFFSET);
        result.extend_from_slice(bytes_of(&(*self as u64)));
        result.extend_from_slice(bytes_of(&params));
        result
    }
}

/// Create a new DEX market
///
/// The asset agnostic orderbook must be properly initialized beforehand.
#[allow(clippy::clippy::too_many_arguments)]
pub fn create_market(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    orderbook: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    aaob_program: Pubkey,
    market_admin: Pubkey,
    create_market_params: create_market::Params,
) -> Instruction {
    let data = DexInstruction::CreateMarket.serialize(create_market_params);
    let accounts = vec![
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(orderbook, false),
        AccountMeta::new_readonly(base_vault, false),
        AccountMeta::new_readonly(quote_vault, false),
        AccountMeta::new_readonly(aaob_program, false),
        AccountMeta::new_readonly(market_admin, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}
/**
Execute a new order on the orderbook.

Depending on the provided parameters, the program will attempt to match the order with existing entries
in the orderbook, and then optionally post the remaining order.
*/
#[allow(clippy::clippy::too_many_arguments)]
pub fn new_order(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    user_account: Pubkey,
    user_token_account: Pubkey,
    user_account_owner: Pubkey,
    discount_account: Option<Pubkey>,
    new_order_params: new_order::Params,
) -> Instruction {
    let data = DexInstruction::NewOrder.serialize(new_order_params);
    let mut accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(user_account_owner, true),
    ];

    if let Some(a) = discount_account {
        accounts.push(AccountMeta::new_readonly(a, false))
    }

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Cancel an existing order and remove it from the orderbook.
#[allow(clippy::clippy::too_many_arguments)]
pub fn cancel_order(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    cancel_order_params: cancel_order::Params,
) -> Instruction {
    let data = DexInstruction::CancelOrder.serialize(cancel_order_params);
    let accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Crank the processing of DEX events.
#[allow(clippy::too_many_arguments)]
pub fn consume_events(
    dex_program_id: Pubkey,
    agnostic_orderbook_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    reward_target: Pubkey,
    user_accounts: &[Pubkey],
    consume_events_params: consume_events::Params,
) -> Instruction {
    let data = DexInstruction::ConsumeEvents.serialize(consume_events_params);
    let mut accounts = vec![
        AccountMeta::new_readonly(agnostic_orderbook_program_id, false),
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(reward_target, false),
    ];

    accounts.extend(user_accounts.iter().map(|k| AccountMeta::new(*k, false)));

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Initialize a new user account
#[allow(clippy::too_many_arguments)]
pub fn initialize_account(
    dex_program_id: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    fee_payer: Pubkey,
    params: initialize_account::Params,
) -> Instruction {
    let data = DexInstruction::InitializeAccount.serialize(params);
    let accounts = vec![
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(fee_payer, true),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Extract accumulated fees from the market. This is an admin instruction
#[allow(clippy::too_many_arguments)]
pub fn sweep_fees(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    market_admin: Pubkey,
    quote_vault: Pubkey,
    destination_token_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::SweepFees.serialize(());
    let accounts = vec![
        AccountMeta::new(market_account, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new_readonly(market_admin, true),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(destination_token_account, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Extract available base and quote token assets from a user account
#[allow(clippy::too_many_arguments)]
pub fn settle(
    dex_program_id: Pubkey,
    market_account: Pubkey,
    market_signer: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    destination_base_account: Pubkey,
    destination_quote_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::Settle.serialize(());
    let accounts = vec![
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(market_account, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new_readonly(market_signer, false),
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(destination_base_account, false),
        AccountMeta::new(destination_quote_account, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Close an inactive and fully settled account
pub fn close_account(
    dex_program_id: Pubkey,
    user_account: Pubkey,
    user_account_owner: Pubkey,
    target_lamports_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::CloseAccount.serialize(());
    let accounts = vec![
        AccountMeta::new(user_account, false),
        AccountMeta::new_readonly(user_account_owner, true),
        AccountMeta::new(target_lamports_account, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}

/// Close an existing market
pub fn close_market(
    dex_program_id: Pubkey,
    market: Pubkey,
    base_vault: Pubkey,
    quote_vault: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    aaob_program: Pubkey,
    market_admin: Pubkey,
    target_lamports_account: Pubkey,
) -> Instruction {
    let data = DexInstruction::CloseAccount.serialize(());
    let accounts = vec![
        AccountMeta::new(market, false),
        AccountMeta::new(base_vault, false),
        AccountMeta::new(quote_vault, false),
        AccountMeta::new(market_signer, false),
        AccountMeta::new(orderbook, false),
        AccountMeta::new(event_queue, false),
        AccountMeta::new(bids, false),
        AccountMeta::new(asks, false),
        AccountMeta::new_readonly(aaob_program, false),
        AccountMeta::new_readonly(market_admin, true),
        AccountMeta::new(target_lamports_account, false),
    ];

    Instruction {
        program_id: dex_program_id,
        accounts,
        data,
    }
}
