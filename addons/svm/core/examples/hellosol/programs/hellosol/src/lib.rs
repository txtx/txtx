use anchor_lang::prelude::*;

declare_id!("BqbXap7GbJXfP42q59Ss2my1iwumLiZBT9fkLFPXwSR2");

#[program]
pub mod hellosol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
