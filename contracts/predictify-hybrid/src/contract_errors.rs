use soroban_sdk::{contracterror};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    Error1 = 1,
    Error2 = 2,
    Error3 = 3,
    Error4 = 4,
    Error5 = 5,
    Error6 = 6,
    Error7 = 7,
    Error8 = 8,
    Error9 = 9,
    Error10 = 10,
    Error11 = 11,
    Error12 = 12,
    Error13 = 13,
    Error14 = 14,
    Error15 = 15,
    Error16 = 16,
    Error17 = 17,
    Error18 = 18,
    Error19 = 19,
    Error20 = 20,
}
