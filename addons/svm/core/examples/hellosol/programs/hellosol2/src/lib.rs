use anchor_lang::prelude::*;

declare_id!("DBu8EDKFnUZSWNggsCZDK4VvPvk8ne9n1kxK1Q3RgSpL");

#[program]
pub mod hellosol2 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
