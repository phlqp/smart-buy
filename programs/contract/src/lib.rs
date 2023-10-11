pub mod instructions;
mod safe_math;
use anchor_lang::prelude::*;
use crate::instructions::*;
pub mod error;
declare_id!("A99MuL2DZrm27VMndJ5u4LzwhEB5BXjYqh5voLnCU6Zg");

#[program]
pub mod contract {
    use super::*;

    pub fn smart_buy<'info>(
        ctx: Context<'_, '_, '_, 'info, SmartBuy<'info>>,
        quote_amount: u64
    ) -> Result<()> {
        handler_buy(&ctx, quote_amount)
    }

    pub fn smart_sell<'info>(
        ctx: Context<'_, '_, '_, 'info, SmartSell<'info>>,
        base_amount: u64
    ) -> Result<()> {
        handler_sell(&ctx, base_amount)
    }
}