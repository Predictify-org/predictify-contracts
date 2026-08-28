import os
import re

directories = ["contracts/predictify-hybrid/src"]
for root, dirs, files in os.walk(directories[0]):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            with open(filepath, "r") as f:
                content = f.read()

            # Find create_market calls that end with &None, &None, &None, ) and add two more &None,
            new_content = re.sub(
                r'(&None,\s*&None,\s*&None,?)(\s*\))',
                r'\1\n            &None,\n            &None,\2',
                content
            )

            # Some might have &0u64 followed by 4 Nones, etc. Let's be safe. Let's just match any create_market with 10 arguments.
            # Actually, `&None,\n            &None,\n            &None,` is a very specific signature.
            # Let's just do that replacement, then compile and see if we missed any.
            if new_content != content:
                with open(filepath, "w") as f:
                    f.write(new_content)
                print(f"Fixed {filepath}")
