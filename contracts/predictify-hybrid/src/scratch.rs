use soroban_sdk::{Env, Symbol, Val, TryFromVal};
use crate::types::Market;

pub fn is_market(env: &Env, key: &Symbol) -> bool {
    let val: Option<Val> = env.storage().persistent().get(key);
    match val {
        Some(v) => Market::try_from_val(env, &v).is_ok(),
        None => false,
    }
}
