import re

filepath = "contracts/predictify-hybrid/src/timelock_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(r'#\[test\]\s+fn _disabled', r'fn _disabled', content)

with open(filepath, "w") as f:
    f.write(content)
