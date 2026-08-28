import re
import os

files_to_patch = []
with open("test_compile_errors.txt") as f:
    for line in f:
        if "this method takes 12 arguments but 10 arguments were supplied" in line or "this method takes 12 arguments but 9 arguments were supplied" in line or "missing fields `dispute_stake_floor`" in line or "with_mut" in line or "unwrap" in line or "3 arguments but 4 arguments were supplied" in line:
            parts = line.split(":")
            if len(parts) >= 3:
                files_to_patch.append(parts[0])

files_to_patch = list(set(files_to_patch))

# Actually, we can just patch all instances of client.create_market( ... )
import glob

def patch_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Find client.create_market( ... )
    # The arguments are comma separated. Let's just find the exact call and add arguments.
    # To do this safely, we count commas inside the parentheses of client.create_market.
    # There are 9 commas (10 arguments). We need to add 2 commas/args.
    # We can write a simple state machine.

    out = []
    idx = 0
    while idx < len(content):
        # find "client.create_market(" or "create_market("
        m = re.search(r'\.create_market\(', content[idx:])
        if not m:
            out.append(content[idx:])
            break
        
        start = idx + m.end()
        out.append(content[idx:start])
        idx = start
        
        # count parens
        parens = 1
        arg_count = 1
        i = idx
        while i < len(content) and parens > 0:
            if content[i] == '(':
                parens += 1
            elif content[i] == ')':
                parens -= 1
                if parens == 0:
                    if arg_count == 10:
                        out.append(content[idx:i])
                        out.append(", &None, &None")
                        idx = i
                    break
            elif content[i] == ',' and parens == 1:
                arg_count += 1
            i += 1

    with open(filepath, 'w') as f:
        f.write("".join(out))

for path in glob.glob("contracts/predictify-hybrid/src/**/*.rs", recursive=True):
    patch_file(path)
