use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as tx_instructions;
use spl_governance::addins::voter_weight::VoterWeightAccountType;
use std::mem::size_of;

#[derive(Accounts)]
#[instruction(voter_bump: u8, voter_weight_record_bump: u8)]
pub struct CreateVoter<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter_bump,
        payer = payer,
        space = 8 + size_of::<Voter>(),
    )]
    pub voter: AccountLoader<'info, Voter>,

    /// The authority controling the voter. Must be the same as the
    /// `governing_token_owner` in the token owner record used with
    /// spl-governance.
    pub voter_authority: Signer<'info>,

    /// The voter weight record is the account that will be shown to spl-governance
    /// to prove how much vote weight the voter has. See update_voter_weight_record.
    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter_authority.key().as_ref()],
        bump = voter_weight_record_bump,
        payer = payer,
        space = size_of::<VoterWeightRecord>(),
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    #[account(address = tx_instructions::ID)]
    pub instructions: UncheckedAccount<'info>,
}

/// Creates a new voter account. There can only be a single voter per
/// voter_authority.
///
/// The user must register with spl-governance using the same voter_authority.
/// Their token owner record will be required for withdrawing funds later.
pub fn create_voter(
    ctx: Context<CreateVoter>,
    voter_bump: u8,
    voter_weight_record_bump: u8,
) -> Result<()> {
    // Forbid creating voter accounts from CPI. The goal is to make automation
    // impossible that weakens some of the limitations intentionally imposed on
    // locked tokens.
    {
        let ixns = ctx.accounts.instructions.to_account_info();
        let current_index = tx_instructions::load_current_index_checked(&ixns)? as usize;
        let current_ixn = tx_instructions::load_instruction_at_checked(current_index, &ixns)?;
        require!(
            current_ixn.program_id == *ctx.program_id,
            ErrorCode::ForbiddenCpi
        );
    }

    // Load accounts.
    let registrar = &ctx.accounts.registrar.load()?;
    let voter_authority = ctx.accounts.voter_authority.key();

    let voter = &mut ctx.accounts.voter.load_init()?;
    voter.voter_bump = voter_bump;
    voter.voter_weight_record_bump = voter_weight_record_bump;
    voter.voter_authority = voter_authority;
    voter.registrar = ctx.accounts.registrar.key();

    let voter_weight_record = &mut ctx.accounts.voter_weight_record;
    voter_weight_record.account_type = VoterWeightAccountType::VoterWeightRecord;
    voter_weight_record.realm = registrar.realm;
    voter_weight_record.governing_token_mint = registrar.realm_governing_token_mint;
    voter_weight_record.governing_token_owner = voter_authority;

    Ok(())
}
