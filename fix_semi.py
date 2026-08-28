filepath = "contracts/predictify-hybrid/src/market_audit_tests.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace('        &String::from_str(&t.env, "idem-key-001"),\n    )\n', '        &String::from_str(&t.env, "idem-key-001"),\n    );\n')

with open(filepath, "w") as f:
    f.write(content)
