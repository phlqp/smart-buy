use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Math Error")]
    MathError,
    #[msg("Price Error")]
    PriceError,

    #[msg("The amount is zero")]
    AmountIsZero,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Cannot get bump")]
    CannotGetBump,
    #[msg("U128 Cannot Convert")]
    U128CannotConvert,
    #[msg("Calculation Failure")]
    CalculationFailure,
}