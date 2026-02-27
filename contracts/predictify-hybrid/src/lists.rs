use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::errors::Error;

/// Simple global access control lists for users and event creators.
///
/// Design:
/// - Empty whitelist means "whitelist disabled" (no restriction).
/// - Blacklist always applies when it contains an address.
/// - For betting, blacklist is checked first, then whitelist (if non-empty).
/// - For event creation, a global creator blacklist is enforced.
pub struct AccessLists;

impl AccessLists {
    const USER_WHITELIST_KEY: &'static str = "UserWhitelist";
    const USER_BLACKLIST_KEY: &'static str = "UserBlacklist";
    const CREATOR_BLACKLIST_KEY: &'static str = "CreatorBlacklist";

    fn get_address_list(env: &Env, key: &'static str) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&Symbol::new(env, key))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn set_address_list(env: &Env, key: &'static str, list: &Vec<Address>) {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, key), list);
    }

    pub fn add_to_user_whitelist(env: &Env, addresses: &Vec<Address>) {
        let mut list = Self::get_address_list(env, Self::USER_WHITELIST_KEY);
        for addr in addresses.iter() {
            if !list.contains(&addr) {
                list.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::USER_WHITELIST_KEY, &list);
    }

    pub fn remove_from_user_whitelist(env: &Env, addresses: &Vec<Address>) {
        let current = Self::get_address_list(env, Self::USER_WHITELIST_KEY);
        let mut filtered = Vec::new(env);
        for addr in current.iter() {
            if !addresses.contains(&addr) {
                filtered.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::USER_WHITELIST_KEY, &filtered);
    }

    pub fn add_to_user_blacklist(env: &Env, addresses: &Vec<Address>) {
        let mut list = Self::get_address_list(env, Self::USER_BLACKLIST_KEY);
        for addr in addresses.iter() {
            if !list.contains(&addr) {
                list.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::USER_BLACKLIST_KEY, &list);
    }

    pub fn remove_from_user_blacklist(env: &Env, addresses: &Vec<Address>) {
        let current = Self::get_address_list(env, Self::USER_BLACKLIST_KEY);
        let mut filtered = Vec::new(env);
        for addr in current.iter() {
            if !addresses.contains(&addr) {
                filtered.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::USER_BLACKLIST_KEY, &filtered);
    }

    pub fn add_to_creator_blacklist(env: &Env, addresses: &Vec<Address>) {
        let mut list = Self::get_address_list(env, Self::CREATOR_BLACKLIST_KEY);
        for addr in addresses.iter() {
            if !list.contains(&addr) {
                list.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::CREATOR_BLACKLIST_KEY, &list);
    }

    pub fn remove_from_creator_blacklist(env: &Env, addresses: &Vec<Address>) {
        let current = Self::get_address_list(env, Self::CREATOR_BLACKLIST_KEY);
        let mut filtered = Vec::new(env);
        for addr in current.iter() {
            if !addresses.contains(&addr) {
                filtered.push_back(addr);
            }
        }
        Self::set_address_list(env, Self::CREATOR_BLACKLIST_KEY, &filtered);
    }

    /// Enforce global user whitelist/blacklist for betting.
    ///
    /// Logic:
    /// - If user is in global blacklist → `Error::UserBlacklisted`
    /// - Else if whitelist is empty → allowed
    /// - Else if whitelist contains user → allowed
    /// - Else → `Error::UserNotWhitelisted`
    pub fn require_user_can_bet(env: &Env, user: &Address) -> Result<(), Error> {
        let blacklist = Self::get_address_list(env, Self::USER_BLACKLIST_KEY);
        if blacklist.contains(user) {
            return Err(Error::UserBlacklisted);
        }

        let whitelist = Self::get_address_list(env, Self::USER_WHITELIST_KEY);
        if whitelist.is_empty() {
            return Ok(());
        }

        if whitelist.contains(user) {
            Ok(())
        } else {
            Err(Error::UserNotWhitelisted)
        }
    }

    /// Enforce global creator blacklist for event creation.
    pub fn require_creator_can_create(env: &Env, creator: &Address) -> Result<(), Error> {
        let blacklist = Self::get_address_list(env, Self::CREATOR_BLACKLIST_KEY);
        if blacklist.contains(creator) {
            Err(Error::CreatorBlacklisted)
        } else {
            Ok(())
        }
    }
}

