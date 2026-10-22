#!/usr/bin/env python3
"""Resolve the merge conflicts in Predictify lib.rs.

Usage:  python3 resolve_conflicts.py path/to/lib.rs
Writes the fixed file in place and keeps a backup at lib.rs.bak
"""
import re
import shutil
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "lib.rs"
src = open(path, encoding="utf-8").read()
shutil.copyfile(path, path + ".bak")

CONFLICT = re.compile(
    r"<<<<<<< [^\n]*\n(.*?)\n=======\n(.*?)\n>>>>>>> [^\n]*\n", re.DOTALL
)

state = {"n": 0}


def resolve(m):
    state["n"] += 1
    ours, theirs = m.group(1), m.group(2)
    n = state["n"]
    if n == 1:
        # Test module declarations: keep both sides.
        return ours + "\n" + theirs + "\n"
    if n in (2, 3):
        # set_dispute_cumulative_stake_cap / vote_on_dispute: take master's body
        # (the *_authorized call is patched in below).
        return theirs + "\n"
    if n in (4, 5):
        # The branch side was the tail of the previous function; master's side
        # adds new functions. Close the previous function, then take master.
        return "    }\n\n" + theirs + "\n"
    raise SystemExit(f"Unexpected extra conflict block #{n}")


src = CONFLICT.sub(resolve, src)
if state["n"] != 5:
    raise SystemExit(f"Expected 5 conflict blocks, found {state['n']}")

# Keep the fix/single-frame-auth intent: call the *_authorized variants
# from inside master's wrappers.
edits = [
    (
        "disputes::DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, reason)",
        "disputes::DisputeManager::process_dispute_authorized(&env, user.clone(), market_id.clone(), stake, reason)",
    ),
    (
        "disputes::DisputeManager::vote_on_dispute(\n            &env, user, market_id.clone(), dispute_id, vote, stake, reason,",
        "disputes::DisputeManager::vote_on_dispute_authorized(\n            &env, user, market_id.clone(), dispute_id, vote, stake, reason,",
    ),
    (
        "disputes::DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())",
        "disputes::DisputeManager::resolve_dispute_authorized(&env, market_id.clone(), admin.clone())",
    ),
    (
        "fees::FeeManager::collect_fees(&env, admin.clone(), market_id.clone())",
        "fees::FeeManager::collect_fees_authorized(&env, admin.clone(), market_id.clone())",
    ),
]
for old, new in edits:
    if src.count(old) != 1:
        raise SystemExit(f"Expected exactly one match for:\n{old}\nfound {src.count(old)}")
    src = src.replace(old, new)

if re.search(r"^(<<<<<<<|=======|>>>>>>>)", src, re.M):
    raise SystemExit("Conflict markers still present")

open(path, "w", encoding="utf-8").write(src)
print("Done. 5 conflicts resolved, 4 call sites switched to *_authorized.")