import re
with open("/Users/solveetcoagula/.gemini/antigravity/brain/7bd2a51a-d10c-4467-8251-aceda3739595/.system_generated/tasks/task-76.log") as f:
    log = f.read()
# find mismatched types at event_archive.rs:1114
import sys
for match in re.finditer(r'error\[E0308\]: mismatched types.*?event_archive\.rs:1114.*?(?=error\[|\Z)', log, re.DOTALL):
    print(match.group(0))
