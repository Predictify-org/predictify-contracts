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
                r'(max_deviation_z_multiple:\s*[^,]+,\s*history_size:\s*[^,]+),(\s*})',
                r'\1,\n                auto_pause_duration_secs: None\2',
                content
            )

            # also handle EventOracleValidationConfig if it has history_size or max_deviation_z_multiple
            new_content = re.sub(
                r'(override_max_staleness_secs:\s*[^,]+,\s*override_max_confidence_bps:\s*[^,]+),(\s*})',
                r'\1,\n                auto_pause_duration_secs: None\2',
                new_content
            )

            if new_content != content:
                with open(filepath, "w") as f:
                    f.write(new_content)
                print(f"Fixed {filepath}")
