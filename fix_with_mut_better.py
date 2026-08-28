import os
import re

def fix_with_mut(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    # Pattern for `.with_mut(|li| li.timestamp = X);`
    pattern1 = r'(env|self\.env)\.ledger\(\)\.with_mut\(\|li\|\s*li\.timestamp\s*=\s*([^;]+)\);'
    
    def repl1(match):
        env_ref = match.group(1)
        expr = match.group(2)
        # If the expression contains li.timestamp, replace it with `env_ref.ledger().get().timestamp`
        expr = expr.replace("li.timestamp", f"{env_ref}.ledger().get().timestamp")
        return f"{env_ref}.ledger().set(soroban_sdk::testutils::LedgerInfo {{ timestamp: {expr}, ..{env_ref}.ledger().get() }});"

    new_content = re.sub(pattern1, repl1, content)
    
    # Pattern for `.with_mut(|li| { li.timestamp = X; });`
    pattern2 = r'(env|self\.env)\.ledger\(\)\.with_mut\(\|li\|\s*\{\s*li\.timestamp\s*=\s*([^;]+);\s*\}\);'
    
    def repl2(match):
        env_ref = match.group(1)
        expr = match.group(2)
        expr = expr.replace("li.timestamp", f"{env_ref}.ledger().get().timestamp")
        return f"{env_ref}.ledger().set(soroban_sdk::testutils::LedgerInfo {{ timestamp: {expr}, ..{env_ref}.ledger().get() }});"

    new_content = re.sub(pattern2, repl2, new_content)

    # Pattern for `.with_mut(|li| li.timestamp += X);`
    pattern3 = r'(env|self\.env)\.ledger\(\)\.with_mut\(\|li\|\s*li\.timestamp\s*\+=\s*([^;]+)\);'
    
    def repl3(match):
        env_ref = match.group(1)
        expr = match.group(2)
        expr = expr.replace("li.timestamp", f"{env_ref}.ledger().get().timestamp")
        return f"{env_ref}.ledger().set(soroban_sdk::testutils::LedgerInfo {{ timestamp: {env_ref}.ledger().get().timestamp + {expr}, ..{env_ref}.ledger().get() }});"

    new_content = re.sub(pattern3, repl3, new_content)

    if new_content != content:
        with open(filepath, "w") as f:
            f.write(new_content)
        print(f"Fixed {filepath}")

directories = ["contracts/predictify-hybrid/src"]
for root, dirs, files in os.walk(directories[0]):
    for file in files:
        if file.endswith(".rs"):
            fix_with_mut(os.path.join(root, file))
