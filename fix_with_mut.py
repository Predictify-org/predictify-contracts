import re

filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

# Fix literal assignments
content = re.sub(
    r'env\.ledger\(\)\.with_mut\(\|li\| li\.timestamp = (\d+)\);',
    r'env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: \1, ..env.ledger().get() });',
    content
)

# Fix saturated_adds if any
content = re.sub(
    r'env\.ledger\(\)\.with_mut\(\|li\|\s*\{\s*li\.timestamp = li\.timestamp\.saturating_add\(([^)]+)\);\s*\}\);',
    r'env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: env.ledger().get().timestamp.saturating_add(\1), ..env.ledger().get() });',
    content
)

with open(filepath, "w") as f:
    f.write(content)
