import os
import re

directories = ["contracts/predictify-hybrid/src"]
for root, dirs, files in os.walk(directories[0]):
    for file in files:
        if file.endswith(".rs"):
            filepath = os.path.join(root, file)
            with open(filepath, "r") as f:
                content = f.read()

            new_content = content.replace("dispute_stake_floor: 0,", "dispute_stake_floor: None,")
            new_content = new_content.replace("max_participants: 100,", "max_participants: None,")

            if new_content != content:
                with open(filepath, "w") as f:
                    f.write(new_content)
                print(f"Fixed {filepath}")
