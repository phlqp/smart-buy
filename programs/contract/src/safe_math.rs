use crate::error::ErrorCode;
use solana_program::msg;
use std::panic::Location;
use anchor_lang::prelude::*;

pub trait SafeMath: Sized {
    fn safe_add(self, rhs: Self) -> Result<Self>;
    fn safe_sub(self, rhs: Self) -> Result<Self>;
    fn safe_mul(self, rhs: Self) -> Result<Self>;
    fn safe_div(self, rhs: Self) -> Result<Self>;
}

macro_rules! checked_impl {
    ($t:ty) => {
        impl SafeMath for $t {
            #[track_caller]
            #[inline(always)]
            fn safe_add(self, v: $t) -> Result<$t> {
                match self.checked_add(v) {
                    Some(result) => Ok(result),
                    None => {
                        let caller = Location::caller();
                        msg!("Math error thrown at {}:{}", caller.file(), caller.line());
                        Err(ErrorCode::MathError.into())
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_sub(self, v: $t) -> Result<$t> {
                match self.checked_sub(v) {
                    Some(result) => Ok(result),
                    None => {
                        let caller = Location::caller();
                        msg!("Math error thrown at {}:{}", caller.file(), caller.line());
                        Err(ErrorCode::MathError.into())
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_mul(self, v: $t) -> Result<$t> {
                match self.checked_mul(v) {
                    Some(result) => Ok(result),
                    None => {
                        let caller = Location::caller();
                        msg!("Math error thrown at {}:{}", caller.file(), caller.line());
                        Err(ErrorCode::MathError.into())
                    }
                }
            }

            #[track_caller]
            #[inline(always)]
            fn safe_div(self, v: $t) -> Result<$t> {
                match self.checked_div(v) {
                    Some(result) => Ok(result),
                    None => {
                        let caller = Location::caller();
                        msg!("Math error thrown at {}:{}", caller.file(), caller.line());
                        Err(ErrorCode::MathError.into())
                    }
                }
            }
        }
    };
}

checked_impl!(u128);
checked_impl!(u64);
checked_impl!(u32);
checked_impl!(u16);
checked_impl!(u8);
checked_impl!(i128);
checked_impl!(i64);
checked_impl!(i32);
checked_impl!(i16);
checked_impl!(i8);

