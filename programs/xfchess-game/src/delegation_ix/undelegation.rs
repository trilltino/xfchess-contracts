use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeAfterUndelegation<'info> {
    #[account(mut)]
    pub base_account: UncheckedAccount<'info>,
    #[account(
        seeds = [b"undelegate-buffer", base_account.key().as_ref()],
        bump,
        seeds::program = ephemeral_rollups_sdk::id(),
    )]
    pub buffer: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
