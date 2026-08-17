#!/usr/bin/env bash
# Post prepared replies on manaflow-ai/cmux#10045 and resolve each thread.
# Requires: gh authenticated as RaviTharuma with write access to manaflow-ai/cmux.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPLIES_JSON="${ROOT}/pr10045-thread-replies.json"
OWNER="manaflow-ai"
REPO="cmux"
PR=10045

LOGIN="$(gh api user --jq .login)"
echo "Authenticated as ${LOGIN}"

# Emit commentId + body pairs as JSON lines
python3 - "$REPLIES_JSON" <<'PY' > /tmp/pr10045-reply-jobs.jsonl
import json, sys
from pathlib import Path
items = json.loads(Path(sys.argv[1]).read_text())
for item in items:
    cid = item.get("commentId")
    if not cid:
        continue
    print(json.dumps({"commentId": cid, "body": item["body"]}))
PY

while IFS= read -r line; do
  comment_id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["commentId"])' "$line")"
  body="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["body"])' "$line")"
  echo "→ reply commentId=${comment_id}"
  # Write body to temp file to avoid shell escaping issues
  printf '%s' "$body" > /tmp/pr10045-reply-body.txt
  gh api \
    -X POST \
    "repos/${OWNER}/${REPO}/pulls/${PR}/comments" \
    -F body=@/tmp/pr10045-reply-body.txt \
    -F in_reply_to="${comment_id}" \
    --jq '{id,in_reply_to_id,html_url}' \
    || echo "WARN: reply failed for ${comment_id}" >&2
done < /tmp/pr10045-reply-jobs.jsonl

echo "Resolving unresolved review threads…"
gh api graphql -f query="
query {
  repository(owner:\"${OWNER}\", name:\"${REPO}\") {
    pullRequest(number:${PR}) {
      reviewThreads(first:100) {
        nodes { id isResolved }
      }
    }
  }
}" --jq '.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved==false) | .id' \
| while read -r thread_id; do
  [ -z "${thread_id}" ] && continue
  echo "→ resolve ${thread_id}"
  gh api graphql -f query='
    mutation($id:ID!) {
      resolveReviewThread(input:{threadId:$id}) {
        thread { isResolved }
      }
    }' -f id="${thread_id}" --jq '.data.resolveReviewThread.thread.isResolved' \
    || echo "WARN: resolve failed for ${thread_id}" >&2
done

echo "Done."
