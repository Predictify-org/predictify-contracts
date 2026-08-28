import re

filepath = "contracts/predictify-hybrid/src/test_audit_trail.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(r'#\[test\]\s+fn _disabled_test_', r'fn _disabled_test_', content)

with open(filepath, "w") as f:
    f.write(content)
