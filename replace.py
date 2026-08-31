import re

content = open('contracts/predictify-hybrid/src/lib.rs').read()

# Pattern 1: unwrap_or_else with panic
content = re.sub(
    r'env\s*\.storage\(\)\s*\.persistent\(\)\s*\.get\(&?market_id\)\s*\.unwrap_or_else\(\|\|\s*\{\s*panic_with_error!\(env,\s*Error::MarketNotFound\);\s*\}\)',
    r'markets::MarketStateManager::get_market(&env, &market_id).unwrap_or_else(|e| panic_with_error!(env, e))',
    content
)

# Pattern 2: ok_or(Error::MarketNotFound)?
content = re.sub(
    r'env\s*\.storage\(\)\s*\.persistent\(\)\s*\.get\(&?market_id\)\s*\.ok_or\(Error::MarketNotFound\)\?',
    r'markets::MarketStateManager::get_market(&env, &market_id)?',
    content
)

# Pattern 3: Option<Market>
content = re.sub(
    r'env\s*\.storage\(\)\s*\.persistent\(\)\s*\.get\(&?market_id\)',
    r'markets::MarketStateManager::get_market(&env, &market_id).ok()',
    content
)

# The third pattern will replace remaining get(&market_id) like line 2379 and 2402

with open('contracts/predictify-hybrid/src/lib.rs', 'w') as f:
    f.write(content)
