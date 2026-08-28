import re
filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

dup = """            dispute_stake_floor: None,
            max_participants: None,
            timelock_config: crate::timelock::MarketTimelockConfig::default(),"""

content = content.replace(f"{dup}\n{dup}", dup)
with open(filepath, "w") as f:
    f.write(content)
