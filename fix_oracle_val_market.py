import re

filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(
    r'(status: MarketStatus::Open,\n\s*)(})',
    r'\1dispute_stake_floor: None,\n            max_participants: None,\n            timelock_config: crate::timelock::MarketTimelockConfig::default(),\n        \2',
    content
)

with open(filepath, "w") as f:
    f.write(content)
