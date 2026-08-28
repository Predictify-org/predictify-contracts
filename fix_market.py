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
                r'(winnings_swept:\s*[^,]+,)(\s*timelock_config:.*?)?(\s*})',
                r'\1\n                dispute_stake_floor: 0,\n                max_participants: 100,\2\3',
                content
            )

            if new_content != content:
                with open(filepath, "w") as f:
                    f.write(new_content)
                print(f"Fixed {filepath}")
