import re

filepath = "contracts/predictify-hybrid/src/governance_tests.rs"
with open(filepath, "r") as f:
    content = f.read()
content = content.replace("timestamp: li.timestamp - 1, ..env.ledger().get()", "timestamp: fixture.env.ledger().get().timestamp - 1, ..fixture.env.ledger().get()")
with open(filepath, "w") as f:
    f.write(content)

filepath = "contracts/predictify-hybrid/src/tie_resolution_tests.rs"
with open(filepath, "r") as f:
    content = f.read()
content = content.replace("timestamp: env.ledger().get().timestamp + 86_401, ..env.ledger().get()", "timestamp: self.env.ledger().get().timestamp + 86_401, ..self.env.ledger().get()")
with open(filepath, "w") as f:
    f.write(content)
