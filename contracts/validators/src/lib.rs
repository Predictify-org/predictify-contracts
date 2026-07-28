//! Validators contract — NatSpec-style rustdoc on every public entrypoint.
//!
//! Provides an on-chain validator registry for the Predictify prediction-market
//! platform.  Validators stake tokens to participate in outcome resolution; the
//! admin controls registration policy and can pause the subsystem in an
//! emergency.
//!
//! # Design overview
//!
//! The contract stores a [`ValidatorInfo`] record per registered address and
//! maintains global counters / configuration in instance storage.  Every
//! state-changing call enforces `require_auth` on the acting address before
//! touching persistent state, so replay attacks and spoofed callers are
//! prevented at the SDK level.
//!
//! # Auth Matrix
//!
//! | Entrypoint              | Required role           |
//! |-------------------------|-------------------------|
//! | `initialize`            | Admin (caller)          |
//! | `register_validator`    | Validator (self)        |
//! | `deregister_validator`  | Validator (self)        |
//! | `update_stake`          | Validator (self)        |
//! | `set_validator_active`  | Admin                   |
//! | `update_score`          | Admin                   |
//! | `update_stake_limits`   | Admin                   |
//! | `pause_validators`      | Admin                   |
//! | `unpause_validators`    | Admin                   |
//! | `transfer_ownership`    | Admin                   |
//! | `get_validator`         | Anyone (read-only)      |
//! | `is_validator`          | Anyone (read-only)      |
//! | `validator_count`       | Anyone (read-only)      |
//! | `is_validators_paused`  | Anyone (read-only)      |
//! | `admin`                 | Anyone (read-only)      |
//! | `version`               | Anyone (read-only)      |

#![no_std]

mod errors;
mod types;

pub use errors::ValidatorError;
pub use types::{DataKey, ValidatorInfo};

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, Env};

/// The Validators contract.
///
/// Register, manage, and query on-chain validators for the Predictify platform.
#[contract]
pub struct ValidatorsContract;

#[contractimpl]
impl ValidatorsContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialise the contract with an `admin` address and stake bounds.
    ///
    /// # What it does
    /// Stores the initial admin, records the initialized flag, and sets the
    /// global minimum/maximum stake limits.  All subsequent admin-gated
    /// entrypoints will check against the stored admin address.
    ///
    /// # How it works
    /// 1. `admin.require_auth()` — the transaction must be signed by `admin`.
    /// 2. Guards against double-initialization by checking [`DataKey::Initialized`].
    /// 3. Persists `admin`, `min_stake`, `max_stake`, and `ValidatorCount = 0`.
    ///
    /// # Why auth is required
    /// Without auth the deployer could be front-run by any account that submits
    /// `initialize` before the legitimate owner.
    ///
    /// # Arguments
    /// * `admin`     — The address that will own this contract instance.
    /// * `min_stake` — Minimum stake in stroops a validator must hold.
    /// * `max_stake` — Maximum stake in stroops a validator may hold.
    ///
    /// # Errors
    /// * [`ValidatorError::AlreadyInitialized`] — contract has already been
    ///   initialized; this call is a no-op after the first invocation.
    /// * [`ValidatorError::InvalidConfig`] — `min_stake > max_stake` or either
    ///   value is negative.
    pub fn initialize(
        env: Env,
        admin: Address,
        min_stake: i128,
        max_stake: i128,
    ) -> Result<(), ValidatorError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(ValidatorError::AlreadyInitialized);
        }
        if min_stake < 0 || max_stake < 0 || min_stake > max_stake {
            return Err(ValidatorError::InvalidConfig);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage()
            .instance()
            .set(&DataKey::ValidatorsPaused, &false);
        env.storage()
            .persistent()
            .set(&DataKey::ValidatorCount, &0u32);
        env.storage().instance().set(&DataKey::MinStake, &min_stake);
        env.storage().instance().set(&DataKey::MaxStake, &max_stake);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Validator lifecycle
    // -----------------------------------------------------------------------

    /// Register `validator` with an initial `stake`.
    ///
    /// # What it does
    /// Creates a new [`ValidatorInfo`] record and increments the global
    /// validator counter.
    ///
    /// # How it works
    /// 1. `validator.require_auth()` — only the validator themselves may
    ///    self-register.
    /// 2. Checks that the contract is initialized and not paused.
    /// 3. Verifies that `stake` is within the `[min_stake, max_stake]` window.
    /// 4. Panics with [`ValidatorError::AlreadyRegistered`] if the address is
    ///    already present.
    /// 5. Writes the new [`ValidatorInfo`] and bumps `ValidatorCount`.
    ///
    /// # Why auth is required
    /// Requiring the validator's own signature prevents griefing where a third
    /// party registers an address without consent, locking a stake slot.
    ///
    /// # Arguments
    /// * `validator` — The Stellar address to register.
    /// * `stake`     — Initial stake amount in stroops.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`]  — call `initialize` first.
    /// * [`ValidatorError::ValidatorsPaused`] — subsystem is paused.
    /// * [`ValidatorError::AlreadyRegistered`] — address already registered.
    /// * [`ValidatorError::StakeTooLow`]  — `stake < min_stake`.
    /// * [`ValidatorError::StakeTooHigh`] — `stake > max_stake`.
    /// * [`ValidatorError::Overflow`]     — validator count would overflow.
    pub fn register_validator(
        env: Env,
        validator: Address,
        stake: i128,
    ) -> Result<(), ValidatorError> {
        validator.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_not_paused(&env)?;

        let min_stake: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MinStake)
            .unwrap_or(0);
        let max_stake: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MaxStake)
            .unwrap_or(i128::MAX);

        if stake < min_stake {
            return Err(ValidatorError::StakeTooLow);
        }
        if stake > max_stake {
            return Err(ValidatorError::StakeTooHigh);
        }
        if env
            .storage()
            .persistent()
            .has(&DataKey::Validator(validator.clone()))
        {
            return Err(ValidatorError::AlreadyRegistered);
        }

        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ValidatorCount)
            .unwrap_or(0);
        let new_count = count
            .checked_add(1)
            .ok_or(ValidatorError::Overflow)?;

        let info = ValidatorInfo {
            address: validator.clone(),
            stake,
            active: true,
            registered_at: env.ledger().sequence(),
            score: 0,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Validator(validator), &info);
        env.storage()
            .persistent()
            .set(&DataKey::ValidatorCount, &new_count);

        Ok(())
    }

    /// Deregister (remove) `validator` from the registry.
    ///
    /// # What it does
    /// Removes the [`ValidatorInfo`] record for `validator` and decrements the
    /// global counter.
    ///
    /// # How it works
    /// 1. `validator.require_auth()` — only the validator themselves may
    ///    self-deregister.
    /// 2. Ensures the contract is initialized and not paused.
    /// 3. Panics with [`ValidatorError::ValidatorNotFound`] if the address is
    ///    unknown.
    /// 4. Removes the persistent record and decrements `ValidatorCount`.
    ///
    /// # Why auth is required
    /// Without auth, any account could remove a validator's registration,
    /// effectively ejecting them from the set.
    ///
    /// # Arguments
    /// * `validator` — The address to deregister.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`]    — contract not yet initialized.
    /// * [`ValidatorError::ValidatorsPaused`]  — subsystem is paused.
    /// * [`ValidatorError::ValidatorNotFound`] — address is not registered.
    pub fn deregister_validator(env: Env, validator: Address) -> Result<(), ValidatorError> {
        validator.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_not_paused(&env)?;

        let key = DataKey::Validator(validator);
        if !env.storage().persistent().has(&key) {
            return Err(ValidatorError::ValidatorNotFound);
        }

        env.storage().persistent().remove(&key);

        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::ValidatorCount)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&DataKey::ValidatorCount, &count.saturating_sub(1));

        Ok(())
    }

    /// Update the stake held by `validator`.
    ///
    /// # What it does
    /// Replaces the existing `stake` field in the validator's [`ValidatorInfo`]
    /// with `new_stake`.
    ///
    /// # How it works
    /// 1. `validator.require_auth()` — only the validator themselves may adjust
    ///    their own stake.
    /// 2. Ensures the contract is initialized and not paused.
    /// 3. Validates `new_stake` against `[min_stake, max_stake]`.
    /// 4. Loads the existing record, updates `stake`, and re-persists it.
    ///
    /// # Why auth is required
    /// Stake is an economic commitment; allowing third parties to update it
    /// would let an attacker reduce a validator's skin-in-the-game.
    ///
    /// # Arguments
    /// * `validator`  — The validator whose stake is being updated.
    /// * `new_stake`  — The replacement stake value in stroops.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`]    — contract not yet initialized.
    /// * [`ValidatorError::ValidatorsPaused`]  — subsystem is paused.
    /// * [`ValidatorError::ValidatorNotFound`] — `validator` is not registered.
    /// * [`ValidatorError::StakeTooLow`]  — `new_stake < min_stake`.
    /// * [`ValidatorError::StakeTooHigh`] — `new_stake > max_stake`.
    pub fn update_stake(
        env: Env,
        validator: Address,
        new_stake: i128,
    ) -> Result<(), ValidatorError> {
        validator.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_not_paused(&env)?;

        let min_stake: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MinStake)
            .unwrap_or(0);
        let max_stake: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MaxStake)
            .unwrap_or(i128::MAX);

        if new_stake < min_stake {
            return Err(ValidatorError::StakeTooLow);
        }
        if new_stake > max_stake {
            return Err(ValidatorError::StakeTooHigh);
        }

        let key = DataKey::Validator(validator);
        let mut info: ValidatorInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ValidatorError::ValidatorNotFound)?;

        info.stake = new_stake;
        env.storage().persistent().set(&key, &info);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Admin-gated management
    // -----------------------------------------------------------------------

    /// Activate or deactivate a validator without removing their record.
    ///
    /// # What it does
    /// Flips the `active` field of the target validator's [`ValidatorInfo`].
    /// An inactive validator is still registered but excluded from resolution
    /// quorum checks.
    ///
    /// # How it works
    /// 1. `admin.require_auth()` — only the current admin may change active
    ///    status.
    /// 2. Verifies the caller is the stored admin address.
    /// 3. Loads the validator record, sets `active = is_active`, persists.
    ///
    /// # Why admin-only
    /// Active status determines inclusion in resolution quorums; a rogue
    /// caller toggling this could stall market resolution.
    ///
    /// # Arguments
    /// * `admin`     — Must be the current contract admin.
    /// * `validator` — The target validator address.
    /// * `is_active` — `true` to activate, `false` to deactivate.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`]    — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]      — `admin` is not the stored admin.
    /// * [`ValidatorError::ValidatorNotFound`] — `validator` is not registered.
    pub fn set_validator_active(
        env: Env,
        admin: Address,
        validator: Address,
        is_active: bool,
    ) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        let key = DataKey::Validator(validator);
        let mut info: ValidatorInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ValidatorError::ValidatorNotFound)?;

        info.active = is_active;
        env.storage().persistent().set(&key, &info);

        Ok(())
    }

    /// Update the performance score of `validator`.
    ///
    /// # What it does
    /// Replaces the `score` field in the validator's [`ValidatorInfo`] with
    /// `new_score`.  The score is an application-defined integer; higher is
    /// better by convention, but the contract itself only stores it.
    ///
    /// # How it works
    /// 1. `admin.require_auth()` — only the current admin may write scores.
    /// 2. Checks admin identity.
    /// 3. Loads the validator's record, updates `score`, persists.
    ///
    /// # Why admin-only
    /// Scores influence off-chain incentive calculations.  Allowing arbitrary
    /// callers to self-report scores would trivially corrupt the leaderboard.
    ///
    /// # Arguments
    /// * `admin`     — Must be the current contract admin.
    /// * `validator` — The validator whose score is being updated.
    /// * `new_score` — Replacement score value.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`]    — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]      — `admin` is not the stored admin.
    /// * [`ValidatorError::ValidatorNotFound`] — `validator` is not registered.
    pub fn update_score(
        env: Env,
        admin: Address,
        validator: Address,
        new_score: i128,
    ) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        let key = DataKey::Validator(validator);
        let mut info: ValidatorInfo = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ValidatorError::ValidatorNotFound)?;

        info.score = new_score;
        env.storage().persistent().set(&key, &info);

        Ok(())
    }

    /// Update the global minimum and maximum stake limits.
    ///
    /// # What it does
    /// Replaces the [`DataKey::MinStake`] and [`DataKey::MaxStake`] values
    /// stored in instance storage.  The new limits apply only to future
    /// `register_validator` / `update_stake` calls; existing validators are
    /// not retroactively deregistered.
    ///
    /// # How it works
    /// 1. `admin.require_auth()` — only the current admin may change limits.
    /// 2. Validates `min_stake <= max_stake` and both non-negative.
    /// 3. Stores the new values in instance storage.
    ///
    /// # Why admin-only
    /// Stake limits are a core security parameter.  Lowering them to zero
    /// would allow zero-stake validators, so this must be gated to the admin.
    ///
    /// # Arguments
    /// * `admin`     — Must be the current contract admin.
    /// * `min_stake` — New minimum stake in stroops (inclusive).
    /// * `max_stake` — New maximum stake in stroops (inclusive).
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`] — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]   — `admin` is not the stored admin.
    /// * [`ValidatorError::InvalidConfig`]  — `min_stake > max_stake` or
    ///   negative value supplied.
    pub fn update_stake_limits(
        env: Env,
        admin: Address,
        min_stake: i128,
        max_stake: i128,
    ) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        if min_stake < 0 || max_stake < 0 || min_stake > max_stake {
            return Err(ValidatorError::InvalidConfig);
        }

        env.storage().instance().set(&DataKey::MinStake, &min_stake);
        env.storage().instance().set(&DataKey::MaxStake, &max_stake);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Pause / Resume
    // -----------------------------------------------------------------------

    /// Pause the validators subsystem.
    ///
    /// # What it does
    /// Sets the [`DataKey::ValidatorsPaused`] flag to `true`, causing all
    /// state-changing entrypoints to return
    /// [`ValidatorError::ValidatorsPaused`] until `unpause_validators` is
    /// called.  Read-only entrypoints are unaffected.
    ///
    /// # How it works
    /// 1. `admin.require_auth()`.
    /// 2. Checks admin identity.
    /// 3. Sets `ValidatorsPaused = true` in instance storage.
    ///
    /// # Why it exists
    /// An emergency pause allows the admin to freeze validator activity during
    /// an incident (e.g. oracle compromise) without deploying a new contract.
    ///
    /// # Arguments
    /// * `admin` — Must be the current contract admin.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`] — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]   — `admin` is not the stored admin.
    pub fn pause_validators(env: Env, admin: Address) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&DataKey::ValidatorsPaused, &true);

        Ok(())
    }

    /// Resume the validators subsystem after a pause.
    ///
    /// # What it does
    /// Clears the [`DataKey::ValidatorsPaused`] flag, re-enabling all
    /// state-changing entrypoints.
    ///
    /// # How it works
    /// 1. `admin.require_auth()`.
    /// 2. Checks admin identity.
    /// 3. Sets `ValidatorsPaused = false` in instance storage.
    ///
    /// # Why it exists
    /// The counterpart to `pause_validators`; once an incident is resolved
    /// the admin restores normal operation without redeployment.
    ///
    /// # Arguments
    /// * `admin` — Must be the current contract admin.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`] — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]   — `admin` is not the stored admin.
    pub fn unpause_validators(env: Env, admin: Address) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&DataKey::ValidatorsPaused, &false);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Ownership
    // -----------------------------------------------------------------------

    /// Transfer contract ownership to a new admin address.
    ///
    /// # What it does
    /// Replaces the stored [`DataKey::Admin`] with `new_owner`, taking effect
    /// immediately.  The previous admin loses all admin privileges upon
    /// successful execution.
    ///
    /// # How it works
    /// 1. `admin.require_auth()` — current admin must sign.
    /// 2. Checks admin identity to guard against replay from a stale session.
    /// 3. Validates that `new_owner` differs from zero (Soroban rejects the
    ///    zero address at the SDK level; we additionally check initialization).
    /// 4. Writes `new_owner` to [`DataKey::Admin`].
    ///
    /// # Why auth is required
    /// Without it, any transaction could reassign ownership.
    ///
    /// # Arguments
    /// * `admin`     — Current admin; must sign the transaction.
    /// * `new_owner` — Replacement admin address.
    ///
    /// # Errors
    /// * [`ValidatorError::NotInitialized`] — contract not yet initialized.
    /// * [`ValidatorError::Unauthorized`]   — `admin` is not the stored admin.
    /// * [`ValidatorError::InvalidNewOwner`] — `new_owner` is the same as
    ///   `admin` (no-op transfer is rejected).
    pub fn transfer_ownership(
        env: Env,
        admin: Address,
        new_owner: Address,
    ) -> Result<(), ValidatorError> {
        admin.require_auth();

        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        if new_owner == admin {
            return Err(ValidatorError::InvalidNewOwner);
        }

        env.storage().instance().set(&DataKey::Admin, &new_owner);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read-only view entrypoints (no auth required)
    // -----------------------------------------------------------------------

    /// Return the [`ValidatorInfo`] for `validator`, or `None` if not
    /// registered.
    ///
    /// # What it does
    /// Performs a single persistent-storage lookup and returns the result
    /// without mutation.
    ///
    /// # Why no auth is needed
    /// On-chain validator info is public by nature; restricting reads would
    /// prevent market contracts from verifying quorum.
    ///
    /// # Arguments
    /// * `validator` — The address to look up.
    ///
    /// # Returns
    /// `Some(ValidatorInfo)` if found, `None` otherwise.
    pub fn get_validator(env: Env, validator: Address) -> Option<ValidatorInfo> {
        env.storage()
            .persistent()
            .get(&DataKey::Validator(validator))
    }

    /// Return whether `validator` is currently registered.
    ///
    /// # What it does
    /// Checks for the existence of the persistent record without loading its
    /// full contents.
    ///
    /// # Why no auth is needed
    /// Existence checks are cheaper than full loads and must be callable by
    /// other contracts in the platform (e.g. the resolution contract).
    ///
    /// # Arguments
    /// * `validator` — The address to test.
    ///
    /// # Returns
    /// `true` if a record exists, `false` otherwise.
    pub fn is_validator(env: Env, validator: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Validator(validator))
    }

    /// Return the total number of currently registered validators.
    ///
    /// # What it does
    /// Reads the [`DataKey::ValidatorCount`] counter from persistent storage.
    ///
    /// # Why no auth is needed
    /// The count is a public aggregate that other contracts use for quorum
    /// threshold calculations.
    ///
    /// # Returns
    /// The current validator count as a `u32`.
    pub fn validator_count(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::ValidatorCount)
            .unwrap_or(0)
    }

    /// Return whether the validators subsystem is currently paused.
    ///
    /// # What it does
    /// Reads the [`DataKey::ValidatorsPaused`] flag from instance storage.
    ///
    /// # Why no auth is needed
    /// Pause state must be observable by off-chain monitors and other
    /// contracts without restriction.
    ///
    /// # Returns
    /// `true` if paused, `false` otherwise.
    pub fn is_validators_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::ValidatorsPaused)
            .unwrap_or(false)
    }

    /// Return the current admin address.
    ///
    /// # What it does
    /// Reads [`DataKey::Admin`] from instance storage.
    ///
    /// # Why no auth is needed
    /// Admin identity is a public parameter queried by governance tooling and
    /// cross-contract calls.
    ///
    /// # Returns
    /// The admin [`Address`].
    ///
    /// # Errors (panics)
    /// Panics with `"not initialized"` if called before `initialize`.
    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .expect("not initialized")
    }

    /// Return the contract version.
    ///
    /// # What it does
    /// Returns the hard-coded version constant, useful for off-chain tooling
    /// to detect deployed contract revisions.
    ///
    /// # Why no auth is needed
    /// Version introspection is always safe to expose publicly.
    ///
    /// # Returns
    /// A `u32` version number.
    pub fn version(_env: Env) -> u32 {
        1
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Assert the contract has been initialized.
    fn assert_initialized(env: &Env) -> Result<(), ValidatorError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(ValidatorError::NotInitialized);
        }
        Ok(())
    }

    /// Assert the subsystem is not paused.
    fn assert_not_paused(env: &Env) -> Result<(), ValidatorError> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::ValidatorsPaused)
            .unwrap_or(false);
        if paused {
            return Err(ValidatorError::ValidatorsPaused);
        }
        Ok(())
    }

    /// Assert `caller` matches the stored admin.
    fn assert_is_admin(env: &Env, caller: &Address) -> Result<(), ValidatorError> {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ValidatorError::NotInitialized)?;
        if stored != *caller {
            panic_with_error!(env, ValidatorError::Unauthorized);
        }
        Ok(())
    }
}
