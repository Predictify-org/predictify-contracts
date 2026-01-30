#![allow(dead_code)]

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // Core errors for batch bet testing
    Unauthorized = 100,
    MarketNotFound = 101,
    MarketClosed = 102,
    InvalidOutcome = 108,
    AlreadyVoted = 109,
    AlreadyBet = 110,
    MarketNotResolved = 112,
    NothingToClaim = 113,
    AlreadyClaimed = 114,
    InsufficientStake = 115,
    InvalidQuestion = 300,
    InvalidOutcomes = 301,
    InvalidInput = 401,
    InvalidFeeConfig = 402,
    InsufficientBalance = 403,
    BatchOperationFailed = 505,
}

impl Error {
    pub fn description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "Unauthorized",
            Error::MarketNotFound => "Market not found",
            Error::MarketClosed => "Market closed",
            Error::InvalidOutcome => "Invalid outcome",
            Error::AlreadyVoted => "Already voted",
            Error::AlreadyBet => "Already bet",
            Error::MarketNotResolved => "Market not resolved",
            Error::NothingToClaim => "Nothing to claim",
            Error::AlreadyClaimed => "Already claimed",
            Error::InsufficientStake => "Insufficient stake",
            Error::InvalidQuestion => "Invalid question",
            Error::InvalidOutcomes => "Invalid outcomes",
            Error::InvalidInput => "Invalid input",
            Error::InvalidFeeConfig => "Invalid fee config",
            Error::InsufficientBalance => "Insufficient balance",
            Error::BatchOperationFailed => "Batch operation failed",
        }
    }
}
