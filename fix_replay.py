filepath = "contracts/predictify-hybrid/tests/event_replay_nonce.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("#![cfg(any())]\nuse soroban_sdk::testutils::Address as _;\n#![cfg(test)]", "#![cfg(any())]\nuse soroban_sdk::testutils::Address as _;")

with open(filepath, "w") as f:
    f.write(content)
