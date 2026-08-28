import re

filepath = "contracts/predictify-hybrid/src/storage.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(r'#\[derive\(Clone, Debug\)\]\npub struct StorageTtlPressure \{[^\}]+\}', r'', content)
content = re.sub(r'/// Pre-flight query to check TTL pressure of keys\s*pub fn check_ttl_pressure\(.*?\)\s*->\s*Vec<StorageTtlPressure>\s*\{.*?\n    \}', r'', content, flags=re.DOTALL)

with open(filepath, "w") as f:
    f.write(content)
