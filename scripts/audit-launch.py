#!/usr/bin/env python3
"""Read-only, heuristic audit of every reachable Git blob. Never print matches."""
import json
import re
import subprocess


def git(*args):
    return subprocess.check_output(["git", *args])


patterns = {
    "private-key": rb"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
    "github-token": rb"gh[pousr]_[A-Za-z0-9]{30,}",
    "service-key": rb"(?:sk-(?:proj-)?[A-Za-z0-9_-]{30,}|AKIA[A-Z0-9]{16})",
    "personal-home-path": rb"/Users/" + rb"m/",
    "email-address": rb"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
}
revisions = git("rev-list", "--all").decode().splitlines()
seen, findings, data_files = set(), [], set()
for revision in revisions:
    for line in git("ls-tree", "-r", revision).decode().splitlines():
        metadata, path = line.split("\t", 1)
        _, _, oid = metadata.split()
        if oid in seen:
            continue
        seen.add(oid)
        if re.search(r"\.(?:db|sqlite|sqlite3|pem|key|p12|ips|log)$", path):
            data_files.add(path)
        blob = git("cat-file", "blob", oid)
        for rule, pattern in patterns.items():
            if re.search(pattern, blob):
                findings.append({"rule": rule, "path": path, "blob": oid[:12]})
print(json.dumps({
    "audited_head": git("rev-parse", "HEAD").decode().strip(),
    "commits": len(revisions),
    "unique_blobs": len(seen),
    "findings_to_review": findings,
    "data_or_secret_file_names": sorted(data_files),
    "scope": "All reachable committed blobs. Heuristics, not a proof of absence.",
}, indent=2))
