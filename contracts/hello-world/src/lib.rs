#![no_std]
use soroban_sdk::{contract, contractimpl, vec, Env, String, Vec};

/// Hello World example contract demonstrating basic Soroban smart contract functionality.
///
/// This contract serves as a simple introduction to Soroban smart contract development
/// and demonstrates fundamental concepts such as contract structure, function implementation,
/// and basic data handling. It provides a minimal but complete example that developers
/// can use as a starting point for building more complex smart contracts.
///
/// # Purpose
///
/// The Hello World contract is designed to:
/// - Demonstrate basic Soroban contract structure and syntax
/// - Show how to implement public contract functions
/// - Illustrate parameter handling and return value construction
/// - Provide a foundation for learning Soroban development
/// - Serve as a template for new contract projects
///
/// # Contract Functions
///
/// - `hello()` - Returns a greeting message with the provided name
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String, Vec};
/// # use hello_world::Contract;
/// # let env = Env::default();
/// # let contract_id = env.register(Contract, ());
/// # let client = hello_world::ContractClient::new(&env, &contract_id);
/// 
/// // Call the hello function
/// let name = String::from_str(&env, "World");
/// let greeting = client.hello(&name);
/// 
/// // greeting will be ["Hello", "World"]
/// assert_eq!(greeting.len(), 2);
/// ```
///
/// # Development Notes
///
/// This contract is intentionally simple and serves as:
/// - A learning resource for new Soroban developers
/// - A template for creating new contracts
/// - A reference implementation for basic contract patterns
/// - A testing ground for development tools and workflows
///
/// For more complex examples and advanced patterns, refer to:
/// - [Soroban Examples Repository](https://github.com/stellar/soroban-examples)
/// - [Stellar Developer Documentation](https://developers.stellar.org/docs/build/smart-contracts/overview)
///
/// # Integration with Predictify Hybrid
///
/// While this contract is independent, it demonstrates the same foundational
/// patterns used in the Predictify Hybrid prediction market system:
/// - Contract structure and organization
/// - Function implementation and parameter handling
/// - Testing patterns and best practices
/// - Documentation standards and conventions
#[contract]
pub struct Contract;

// This is a sample contract. Replace this placeholder with your own contract logic.
// A corresponding test example is available in `test.rs`.
//
// For comprehensive examples, visit <https://github.com/stellar/soroban-examples>.
// The repository includes use cases for the Stellar ecosystem, such as data storage on
// the blockchain, token swaps, liquidity pools, and more.
//
// Refer to the official documentation:
// <https://developers.stellar.org/docs/build/smart-contracts/overview>.
#[contractimpl]
impl Contract {
    /// Generate a friendly greeting message.
    ///
    /// This function demonstrates basic Soroban contract functionality by accepting
    /// a name parameter and returning a greeting message as a vector of strings.
    /// It showcases fundamental concepts including parameter handling, string
    /// manipulation, and vector construction within the Soroban environment.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment providing access to blockchain context
    /// * `to` - The name or identifier to include in the greeting message
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the greeting message components:
    /// - First element: "Hello" (static greeting)
    /// - Second element: The provided `to` parameter
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, String, vec};
    /// # use hello_world::Contract;
    /// # let env = Env::default();
    /// # let contract_id = env.register(Contract, ());
    /// # let client = hello_world::ContractClient::new(&env, &contract_id);
    /// 
    /// // Basic greeting
    /// let name = String::from_str(&env, "Alice");
    /// let result = client.hello(&name);
    /// 
    /// // Verify the result
    /// assert_eq!(result, vec![
    ///     &env,
    ///     String::from_str(&env, "Hello"),
    ///     String::from_str(&env, "Alice")
    /// ]);
    /// 
    /// // Different names produce different greetings
    /// let dev_greeting = client.hello(&String::from_str(&env, "Developer"));
    /// let world_greeting = client.hello(&String::from_str(&env, "World"));
    /// 
    /// // Both contain "Hello" as the first element
    /// assert_eq!(dev_greeting.get(0).unwrap(), String::from_str(&env, "Hello"));
    /// assert_eq!(world_greeting.get(0).unwrap(), String::from_str(&env, "Hello"));
    /// ```
    ///
    /// # Implementation Details
    ///
    /// The function uses the `vec!` macro to construct a vector containing:
    /// 1. A static "Hello" string created using `String::from_str()`
    /// 2. The input `to` parameter passed directly
    ///
    /// This demonstrates:
    /// - **Environment Usage**: Accessing the Soroban environment for string creation
    /// - **Vector Construction**: Building return values using Soroban's vector type
    /// - **String Handling**: Working with Soroban's String type
    /// - **Parameter Processing**: Accepting and using function parameters
    ///
    /// # Learning Objectives
    ///
    /// This function teaches:
    /// - Basic Soroban function signature patterns
    /// - Environment parameter usage and importance
    /// - String and vector manipulation in Soroban
    /// - Return value construction and formatting
    /// - Testing patterns for contract functions
    ///
    /// # Extension Ideas
    ///
    /// Developers can extend this function to:
    /// - Add input validation for the `to` parameter
    /// - Support multiple languages or greeting formats
    /// - Include timestamps or additional metadata
    /// - Implement more complex string processing
    /// - Add logging or event emission
    pub fn hello(env: Env, to: String) -> Vec<String> {
        vec![&env, String::from_str(&env, "Hello"), to]
    }
}

mod test;
