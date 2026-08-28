import os
import re

filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

new_content = re.sub(
    r'(max_participants:\s*100,)(\s*})',
    r'\1\n                timelock_config: None,\2',
    content
)

with open(filepath, "w") as f:
    f.write(new_content)
print(f"Fixed {filepath}")
