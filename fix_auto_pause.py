import re

filepath = "contracts/predictify-hybrid/src/tests/oracle_validation_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(r'(max_staleness_secs: 10,\n\s*)(})', r'\1auto_pause_duration_secs: None,\n\2', content)

with open(filepath, "w") as f:
    f.write(content)
