import re
with open("/Users/solveetcoagula/.gemini/antigravity/brain/7bd2a51a-d10c-4467-8251-aceda3739595/.system_generated/tasks/task-76.log") as f:
    lines = f.readlines()
errors = []
for i, line in enumerate(lines):
    if "error[" in line:
        filename_line = lines[i+1].strip()
        errors.append(f"{line.strip()} at {filename_line}")

for e in sorted(list(set(errors))):
    print(e)
