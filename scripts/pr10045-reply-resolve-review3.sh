#!/usr/bin/env bash
# Reply to + resolve all CodeRabbit review3 threads on manaflow-ai/cmux#10045.
# Run as RaviTharuma with a PAT that can write on the upstream PR:
#   gh auth status   # must be RaviTharuma, not cursor[bot]
#   ./scripts/pr10045-reply-resolve-review3.sh
set -euo pipefail

REPO=manaflow-ai/cmux
PR=10045
REPLIES_URL=${REPLIES_URL:-https://raw.githubusercontent.com/RaviTharuma/cmux-herdr/cursor/pr10045-coderabbit-replies-f1c1/docs/upstream/patches/pr10045-review3-replies.json}

tmp=$(mktemp)
curl -fsSL "$REPLIES_URL" -o "$tmp"
echo "Loaded $(python3 -c "import json;print(len(json.load(open('$tmp'))))") replies"

python3 - "$tmp" <<'PY'
import json, subprocess, sys, time
path = sys.argv[1]
items = json.load(open(path))
ok_reply = ok_resolve = fail = 0
for i, item in enumerate(items, 1):
    body = item["body"]
    db = item["db_id"]
    thread = item["thread_id"]
    print(f"[{i}/{len(items)}] reply comment={db} …", flush=True)
    r = subprocess.run(
        ["gh","api",f"repos/manaflow-ai/cmux/pulls/10045/comments/{db}/replies",
         "-f",f"body={body}"],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        # fallback: create reply via pulls comments with in_reply_to
        r = subprocess.run(
            ["gh","api","repos/manaflow-ai/cmux/pulls/10045/comments",
             "-f",f"body={body}","-F",f"in_reply_to={db}"],
            capture_output=True, text=True,
        )
    if r.returncode == 0:
        ok_reply += 1
    else:
        fail += 1
        print("  reply failed:", r.stderr[:200], flush=True)
        continue
    print(f"  resolve thread={thread[-12:]} …", flush=True)
    q = f'mutation {{ resolveReviewThread(input: {{threadId: "{thread}"}}) {{ thread {{ isResolved }} }} }}'
    r2 = subprocess.run(
        ["gh","api","graphql","-f",f"query={q}"],
        capture_output=True, text=True,
    )
    if r2.returncode == 0 and '"isResolved":true' in r2.stdout.replace(" ",""):
        ok_resolve += 1
    elif r2.returncode == 0 and "isResolved" in r2.stdout:
        ok_resolve += 1
        print("  resolve ok", flush=True)
    else:
        fail += 1
        print("  resolve failed:", r2.stderr[:200] or r2.stdout[:200], flush=True)
    time.sleep(0.4)
print(f"done replies={ok_reply} resolves={ok_resolve} failures={fail}")
PY
