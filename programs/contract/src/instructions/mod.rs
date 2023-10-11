pub mod buy;
use std::mem::size_of;

pub use buy::*;
pub mod sell;
use openbook_dex::{state::Market, critbit::SlabView};
use phoenix::{program::MarketHeader, quantities::WrapperU64};
pub use sell::*;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::account_info::AccountInfo;
use sokoban::ZeroCopy;
use anchor_lang::solana_program::program_error::ProgramError;
use crate::safe_math::SafeMath;

pub fn phoenix_price_in_ticks_to_taker_price(
    price_in_ticks: u64,
    tick_size: u64,
    header: &MarketHeader,
) -> Result<u64> {
    price_in_ticks
        .safe_mul(tick_size)?
        .safe_mul(header.get_quote_lot_size().as_u64())?
        .safe_div(header.raw_base_units_per_base_unit as u64)
}

pub fn calculate_price_from_openbook_limit_price(
    limit_price: u64,
    pc_lot_size: u64,
    coin_lot_size: u64,
) -> Result<u64> {
    Ok((limit_price as u128)
        .safe_mul(pc_lot_size as u128)?
        .safe_div(coin_lot_size as u128)? as u64)
}

pub fn get_phoenix_best_price<'a>(acc: &'a AccountInfo) -> anchor_lang::Result<(Option<u64>, Option<u64>)> {
    let phoenix_market_data = acc.try_borrow_data()?;
    let (header_bytes, bytes) = phoenix_market_data.split_at(size_of::<MarketHeader>());
    let header = MarketHeader::load_bytes(header_bytes).unwrap();
    let market = phoenix::program::load_with_dispatch(&header.market_size_params, bytes)?.inner;

    let best_bid = market.get_book(phoenix::state::Side::Bid).iter().next().and_then(|(o, _)| {
        phoenix_price_in_ticks_to_taker_price(
            o.price_in_ticks.as_u64(),
            market.get_tick_size().as_u64(),
            header,
        ).ok()
    });

    let best_ask = market.get_book(phoenix::state::Side::Ask).iter().next().and_then(|(o, _)| {
        phoenix_price_in_ticks_to_taker_price(
            o.price_in_ticks.as_u64(),
            market.get_tick_size().as_u64(),
            header,
        ).ok()
    });

    Ok((best_bid, best_ask))
}

pub fn get_openbook_best_price<'a>(
    openbook_market: &Market,
    openbook_market_bids: &'a AccountInfo,
    openbook_market_asks: &'a AccountInfo
) -> anchor_lang::Result<(Option<u64>, Option<u64>)> {
    let asks = openbook_market.load_asks_mut(openbook_market_asks).map_err(ProgramError::from)?;
    let bids = openbook_market.load_bids_mut(openbook_market_bids).map_err(ProgramError::from)?;

        let best_bid = match bids.find_max() {
            Some(best_bid_h) => {
                let best_bid_ref = bids
                    .get(best_bid_h)
                    .unwrap()
                    .as_leaf()
                    .unwrap();

                let price = calculate_price_from_openbook_limit_price(
                    best_bid_ref.price().get(),
                    openbook_market.pc_lot_size,
                    openbook_market.coin_lot_size,
                )?;

                Some(price)
            }
            None => None,
        };

        let best_ask = match asks.find_max() {
            Some(best_ask_h) => {
                let best_ask_ref = asks
                    .get(best_ask_h)
                    .unwrap()
                    .as_leaf()
                    .unwrap();

                let price = calculate_price_from_openbook_limit_price(
                    best_ask_ref.price().get(),
                    openbook_market.pc_lot_size,
                    openbook_market.coin_lot_size,
                )?;

                Some(price)
            }
            None => None,
        };

    Ok((best_bid, best_ask))
}