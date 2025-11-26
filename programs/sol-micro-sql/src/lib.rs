use anchor_lang::prelude::*;

declare_id!("9jJqjrdiJTYo9vYftpxJoLrLeuBn2qEQEX8Au1P8r1Gj");

#[program]
pub mod sol_micro_sql {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn process_string(_ctx: Context<ProcessString>, input: String) -> Result<()> {
        msg!("Received string: {}", input);
        emit!(StringProcessed {
            input: input.clone(),
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct ProcessString {}

#[event]
pub struct StringProcessed {
    pub input: String,
}
