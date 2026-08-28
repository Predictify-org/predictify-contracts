import os
import re

directories = ["contracts/predictify-hybrid/src"]
for root, dirs, files in os.walk(directories[0]):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            with open(filepath, "r") as f:
                content = f.read()

            new_content = re.sub(
                r'env\.ledger\(\)\.with_mut\(\|li\| li\.timestamp = ([^;]+)\);',
                r'env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: \1, ..env.ledger().get() });',
                content
            )

            new_content = re.sub(
                r'self\.env\.ledger\(\)\.with_mut\(\|li\| li\.timestamp = ([^;]+)\);',
                r'self.env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: \1, ..self.env.ledger().get() });',
                new_content
            )

            new_content = re.sub(
                r'env\.ledger\(\)\.with_mut\(\|li\| li\.timestamp \+= ([^;]+)\);',
                r'env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: env.ledger().get().timestamp + \1, ..env.ledger().get() });',
                new_content
            )

            new_content = re.sub(
                r'self\.env\.ledger\(\)\.with_mut\(\|li\| li\.timestamp \+= ([^;]+)\);',
                r'self.env.ledger().set(soroban_sdk::testutils::LedgerInfo { timestamp: self.env.ledger().get().timestamp + \1, ..self.env.ledger().get() });',
                new_content
            )

            if new_content != content:
                with open(filepath, "w") as f:
                    f.write(new_content)
                print(f"Fixed {filepath}")
