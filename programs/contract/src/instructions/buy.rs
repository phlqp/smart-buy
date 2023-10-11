use std::num::NonZeroU64;
use anchor_spl::token::Mint;
use phoenix::quantities::WrapperU64;
use anchor_lang::{prelude::*, solana_program::{log::sol_log, program::invoke_signed}};
use crate::error::ErrorCode;

use super::{get_openbook_best_price, get_phoenix_best_price};

pub fn handler_buy<'info>(
    ctx: &Context<'_, '_, '_, 'info, SmartBuy<'info>>,
    quote_amount: u64 // Ex: USDC,USDT,..
) -> Result<()> {
    // Retrieve the best ask and bid prices of the Phoenix's market
    let (phoenix_best_ask_price, phoenix_best_bid_price) = get_phoenix_best_price(&ctx.accounts.phoenix_market)?;

    if phoenix_best_bid_price.is_none() || phoenix_best_ask_price.is_none() {
        msg!("Can not get the price from the Phoenix market");
        return Err(ErrorCode::PriceError.into());
    }

    // Retrieve the best ask and bid prices of the OpenBook's market
    let openbook_market = openbook_dex::state::Market::load(&ctx.accounts.openbook_market, ctx.accounts.openbook_program_id.key, true).map_err(ProgramError::from)?;
    let (openbook_best_bid_price, openbook_best_ask_price) = get_openbook_best_price(&openbook_market, &ctx.accounts.openbook_market_bids, &ctx.accounts.openbook_market_asks)?;

    if openbook_best_bid_price.is_none() || openbook_best_ask_price.is_none() {
        msg!("Can not get the price from the OpenBook market");
        return Err(ErrorCode::PriceError.into());
    }

    let phoenix_best_bid_price = phoenix_best_bid_price.unwrap();
    let phoenix_best_ask_price = phoenix_best_ask_price.unwrap();
    let openbook_best_bid_price = openbook_best_bid_price.unwrap();
    let openbook_best_ask_price = openbook_best_ask_price.unwrap();

    // Write in JSON format for future parsing
    sol_log(&format!(r#"PRICE:{{"phoenix":{{"ask":{},"bid":{}}},"openbook:{{"ask":{},"bib":{}}}}}"#, phoenix_best_ask_price, phoenix_best_bid_price, openbook_best_ask_price, openbook_best_bid_price));

    // Retrieve the balances of the tokens before making a purchase.
    let base_token_balance_before = anchor_spl::token::accessor::amount(&ctx.accounts.base_account)?;
    let quote_token_balance_before = anchor_spl::token::accessor::amount(&ctx.accounts.quote_account)?;

    let per_coin = 10u64.pow(ctx.accounts.base_mint.decimals.into());

    // Buy from OpenBook
    if openbook_best_ask_price <= phoenix_best_ask_price {
        // Call an IOC instruction
        invoke_signed(&
            openbook_dex::instruction::new_order(
                ctx.accounts.openbook_market.key,
                ctx.accounts.openbook_open_orders.key,
                ctx.accounts.openbook_request_queue.key,
                ctx.accounts.openbook_event_queue.key,
                ctx.accounts.openbook_market_bids.key,
                ctx.accounts.openbook_market_asks.key,
                ctx.accounts.quote_account.key, // order_payer
                ctx.accounts.owner.key, // open_orders_account_owner
                ctx.accounts.openbook_coin_vault.key,
                ctx.accounts.openbook_pc_vault.key,
                ctx.accounts.token_program_id.key,
                ctx.accounts.rent_sysvar_id.key, // rent_sysvar_id,
                Option::None, // srm_account_referral,
                ctx.accounts.openbook_program_id.key, // program_id,
                openbook_dex::matching::Side::Bid,
                NonZeroU64::new(openbook_best_ask_price/openbook_market.pc_lot_size).unwrap(), // price
                NonZeroU64::new(quote_amount*per_coin/openbook_best_ask_price/openbook_market.coin_lot_size).unwrap(), // max_coin_qty
                openbook_dex::matching::OrderType::ImmediateOrCancel,
                0,
                openbook_dex::instruction::SelfTradeBehavior::DecrementTake,
                u16::MAX,
                NonZeroU64::new(quote_amount).unwrap(),
                i64::MAX,
            ).map_err(ProgramError::from)?,
            &mut ctx.accounts.to_account_infos(),
            &[]
        )?;

        // Call a settle instruction
        invoke_signed(&
            openbook_dex::instruction::settle_funds(
                ctx.accounts.openbook_program_id.key,
                ctx.accounts.openbook_market.key,
                ctx.accounts.token_program_id.key,
                ctx.accounts.openbook_open_orders.key,
                ctx.accounts.owner.key,
                ctx.accounts.openbook_coin_vault.key,
                ctx.accounts.base_account.key,
                ctx.accounts.openbook_pc_vault.key,
                ctx.accounts.quote_account.key,
                Option::Some(ctx.accounts.quote_account.key),
                ctx.accounts.openbook_vault_signer.key
            ).map_err(ProgramError::from)?,
            &mut ctx.accounts.to_account_infos(),
            &[]
        )?;
    }
    else {
        invoke_signed(
            &phoenix::program::create_new_order_instruction_with_custom_token_accounts(
                ctx.accounts.phoenix_market.key,
                ctx.accounts.owner.key,
                ctx.accounts.base_account.key,
                ctx.accounts.quote_account.key,
                &ctx.accounts.base_mint.key(),
                &ctx.accounts.quote_mint.key(),
                &phoenix::state::OrderPacket::ImmediateOrCancel {
                    side: phoenix::state::Side::Bid,
                    num_base_lots: phoenix::quantities::BaseLots::new(0),
                    num_quote_lots: phoenix::quantities::QuoteLots::new(quote_amount),
                    min_base_lots_to_fill: phoenix::quantities::BaseLots::new(0),
                    min_quote_lots_to_fill: phoenix::quantities::QuoteLots::new(0),
                    self_trade_behavior: phoenix::state::SelfTradeBehavior::CancelProvide,
                    client_order_id: 0,
                    use_only_deposited_funds: false,
                    price_in_ticks: Option::None,
                    match_limit: Option::None,
                    last_valid_slot: Option::None,
                    last_valid_unix_timestamp_in_seconds: Option::None,
                }
            ),
            &mut ctx.accounts.to_account_infos(),
            &[]
        )?;
    }

    // Retrieve the balances of the tokens after making a purchase.
    let base_token_balance_after = anchor_spl::token::accessor::amount(&ctx.accounts.base_account)?;
    let quote_token_balance_after = anchor_spl::token::accessor::amount(&ctx.accounts.quote_account)?;

    if base_token_balance_after > base_token_balance_before {
        let base_amount = base_token_balance_after - base_token_balance_before;
        let bought_quote_amount = quote_token_balance_before - quote_token_balance_after;
        let bought_price = bought_quote_amount * per_coin / base_amount;

        // Write in JSON format for future parsing
        sol_log(&format!(r#"BOUGHT:{{"amount":{},"spend":{},"price":{}}}"#, base_amount, bought_quote_amount, bought_price));
    }
    else {
        msg!("BUY:Fail")
    }

    Ok(())
}

#[derive(Accounts)]
pub struct SmartBuy<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    ///  CHECK:
    pub base_mint: Account<'info, Mint>,
    ///  CHECK:
    pub quote_mint: Account<'info, Mint>,
    /// CHECK:
    #[account(mut)]
    pub base_account: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub quote_account: AccountInfo<'info>,
    ///  CHECK:
    pub phoenix_program_id: AccountInfo<'info>,
    ///  CHECK:
    pub phoenix_log_authority: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub phoenix_market: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub phoenix_base_vault: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub phoenix_quote_vault: AccountInfo<'info>,
    ///  CHECK:
    pub openbook_program_id: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_market: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_request_queue: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_event_queue: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_market_bids: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_market_asks: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_coin_vault: AccountInfo<'info>,
    ///  CHECK:
    #[account(mut)]
    pub openbook_pc_vault: AccountInfo<'info>,
    ///  CHECK:
    pub openbook_vault_signer: AccountInfo<'info>,
    ///  CHECK:
    pub openbook_open_orders: AccountInfo<'info>,
    ///  CHECK:
    pub rent_sysvar_id: AccountInfo<'info>,
    ///  CHECK:
    pub token_program_id: AccountInfo<'info>
}