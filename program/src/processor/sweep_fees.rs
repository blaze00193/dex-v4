use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    error::DexError,
    state::DexState,
    utils::{check_account_key, check_account_owner, check_signer},
};

struct Accounts<'a, 'b: 'a> {
    market: &'a AccountInfo<'b>,
    market_signer: &'a AccountInfo<'b>,
    market_admin: &'a AccountInfo<'b>,
    quote_vault: &'a AccountInfo<'b>,
    destination_token_account: &'a AccountInfo<'b>,
    spl_token_program: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Accounts<'a, 'b> {
    pub fn parse(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let a = Self {
            market: next_account_info(accounts_iter)?,
            market_signer: next_account_info(accounts_iter)?,
            market_admin: next_account_info(accounts_iter)?,
            quote_vault: next_account_info(accounts_iter)?,
            destination_token_account: next_account_info(accounts_iter)?,
            spl_token_program: next_account_info(accounts_iter)?,
        };

        check_signer(a.market_admin).map_err(|e| {
            msg!("The market admin should be a signer for this instruction!");
            e
        })?;
        check_account_key(
            a.spl_token_program,
            &spl_token::ID.to_bytes(),
            DexError::InvalidSplTokenProgram,
        )?;
        check_account_owner(a.market, program_id, DexError::InvalidStateAccountOwner)?;

        Ok(a)
    }
}

pub(crate) fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts = Accounts::parse(program_id, accounts)?;

    let mut market_state = DexState::get(accounts.market)?;
    check_accounts(program_id, &market_state, &accounts)?;

    let transfer_instruction = spl_token::instruction::transfer(
        &spl_token::ID,
        accounts.quote_vault.key,
        accounts.destination_token_account.key,
        accounts.market_signer.key,
        &[],
        market_state.accumulated_fees,
    )?;

    if market_state.accumulated_fees == 0 {
        msg!("There are no fees to be extracted from the market");
        return Err(DexError::NoOp.into());
    }

    invoke_signed(
        &transfer_instruction,
        &[
            accounts.spl_token_program.clone(),
            accounts.quote_vault.clone(),
            accounts.destination_token_account.clone(),
            accounts.market_signer.clone(),
        ],
        &[&[
            &accounts.market.key.to_bytes(),
            &[market_state.signer_nonce as u8],
        ]],
    )?;

    market_state.accumulated_fees = 0;

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
            &[market_state.signer_nonce as u8],
        ],
        program_id,
    )?;
    check_account_key(
        accounts.market_signer,
        &market_signer.to_bytes(),
        DexError::InvalidMarketSignerAccount,
    )?;
    check_account_key(
        accounts.quote_vault,
        &market_state.quote_vault,
        DexError::InvalidQuoteVaultAccount,
    )?;
    check_account_key(
        accounts.market_admin,
        &market_state.admin,
        DexError::InvalidMarketAdminAccount,
    )?;
    Ok(())
}
