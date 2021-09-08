use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{DexState, UserAccount},
    utils::{check_account_key, check_signer},
};

/**
The required arguments for a create_market instruction.
*/
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Params {}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

struct Accounts<'a, 'b: 'a> {
    aaob_program: &'a AccountInfo<'b>,
    spl_token_program: &'a AccountInfo<'b>,
    market: &'a AccountInfo<'b>,
    base_vault: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_owner: &'a AccountInfo<'b>,
    destination_base_account: &'a AccountInfo<'b>,
    destination_quote_account: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            aaob_program: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
            market: next_account_info(accounts_iter)?,
            base_vault: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            user: next_account_info(accounts_iter)?,
            user_owner: next_account_info(accounts_iter)?,
            destination_base_account: next_account_info(accounts_iter)?,
            destination_quote_account: next_account_info(accounts_iter)?,
        };
        check_signer(&a.user_owner).unwrap();
        check_account_key(&a.spl_token_program, &spl_token::ID).unwrap();

        Ok(a)
    }

    pub fn load_user_account(&self) -> Result<UserAccount<'b>, ProgramError> {
        let user_account = UserAccount::parse(&self.user)?;
        if &user_account.header.owner != self.user_owner.key {
            msg!("Invalid user account owner provided!");
            return Err(ProgramError::InvalidArgument);
        }
        if &user_account.header.market != self.market.key {
            msg!("The provided user account doesn't match the current market");
            return Err(ProgramError::InvalidArgument);
        };
        Ok(user_account)
    }
}

pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: Params,
) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let Params {} = params;

    let market_state =
        DexState::deserialize(&mut (&accounts.market.data.borrow() as &[u8]))?.check()?;

    let mut user_account = accounts.load_user_account()?;

    let mut market_data: &mut [u8] = &mut accounts.market.data.borrow_mut();
    market_state.serialize(&mut market_data).unwrap();

    check_accounts(program_id, &market_state, &accounts).unwrap();

    let transfer_quote_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &market_state.quote_vault,
        &accounts.destination_quote_account.key,
        &accounts.market_signer.key,
        &[],
        user_account.header.quote_token_free,
    )?;

    invoke_signed(
        &transfer_quote_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.quote_vault.clone(),
            accounts.destination_quote_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;

    let transfer_base_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        &market_state.base_vault,
        &accounts.destination_base_account.key,
        &accounts.market_signer.key,
        &[],
        user_account.header.base_token_free,
    )?;

    invoke_signed(
        &transfer_base_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.base_vault.clone(),
            accounts.destination_base_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ]],
    )?;

    user_account.header.quote_token_free = 0;
    user_account.header.base_token_free = 0;

    user_account.write();

    Ok(())
}

fn check_accounts(
    program_id: &Pubkey,
    market_state: &DexState,
    accounts: &Accounts,
) -> ProgramResult {
    let market_signer = Pubkey::create_program_address(
        &[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce],
        ],
        program_id,
    )?;
    check_account_key(accounts.market_signer, &market_signer).unwrap();
    check_account_key(accounts.base_vault, &market_state.base_vault).unwrap();
    check_account_key(accounts.quote_vault, &market_state.quote_vault).unwrap();
    check_account_key(accounts.aaob_program, &market_state.aaob_program).unwrap();

    Ok(())
}
