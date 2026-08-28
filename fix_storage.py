import re

filepath = "contracts/predictify-hybrid/src/storage.rs"
with open(filepath, "r") as f:
    content = f.read()

content = re.sub(r'#\[contracttype\]\n#\[derive\(Clone, Debug\)\]\npub struct StorageTtlPressure', r'#[derive(Clone, Debug)]\npub struct StorageTtlPressure', content)

with open(filepath, "w") as f:
    f.write(content)
