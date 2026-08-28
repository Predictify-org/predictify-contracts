filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("winnings_swept: false,", "winnings_swept: false,\n            dispute_stake_floor: None,\n            max_participants: None,\n            timelock_config: crate::timelock::MarketTimelockConfig::default(),")

with open(filepath, "w") as f:
    f.write(content)
