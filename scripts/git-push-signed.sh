#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Create and push a GitHub-verified (`VALID` web-flow signature) commit on the
# active feature branch via GitHub GraphQL `createCommitOnBranch`.
#
# Why this exists:
#   Org ruleset 24634124 permits pushing unsigned commits to `arena/*` feature
#   branches, whereas org ruleset 18225024 (`EstateBranching` on `main`) blocks
#   merging any PR containing unsigned commits, and GitHub's REST `/git/commits`
#   endpoint leaves commits unsigned (`signature: null`). GraphQL's
#   `createCommitOnBranch` signs every commit with GitHub's `web-flow` key
#   (`signature.isValid == true`) without requiring a local GPG agent.
#
# Note:
#   Stage newly created files explicitly with `git add <file>` first; this
#   script inspects only tracked/staged paths (`git ls-files --cached --modified`)
#   so local untracked scratch artifacts are never published automatically.
#
# Usage:
#   git add <files>
#   ./scripts/git-push-signed.sh "feat(scope): short summary" ["optional body / trailers"]

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
export PATH="$HOME/.bun/bin:/usr/local/bin:$PATH"

HEADLINE="${1:-}"
BODY="${2:-}"

# Print usage instructions and exit with status 2 when arguments are missing.
usage_exit() {
  printf 'Usage: %s "<conventional-commit-headline>" ["<optional-body>"]\n' "$0" >&2
  exit 2
}

[ -n "$HEADLINE" ] || usage_exit

if ! command -v bun >/dev/null 2>&1; then
  printf 'git-push-signed: ERROR: bun is required on PATH\n' >&2
  exit 1
fi
if ! command -v gh >/dev/null 2>&1; then
  printf 'git-push-signed: ERROR: gh CLI is required on PATH\n' >&2
  exit 1
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" = "main" ] || [ "$BRANCH" = "master" ] || [ "$BRANCH" = "HEAD" ]; then
  printf 'git-push-signed: ERROR: refusing to commit directly onto %s\n' "$BRANCH" >&2
  exit 1
fi

# Run all local preflight gates before touching GitHub.
bash ./scripts/preflight.sh

git fetch origin --prune

# Enforce the exemption-ratchet trailer locally whenever root-allow.txt changes.
if ! git diff --quiet origin/main -- .machine_readable/root-allow.txt 2>/dev/null \
  || ! git diff --cached --quiet origin/main -- .machine_readable/root-allow.txt 2>/dev/null; then
  FULL_MSG="${HEADLINE}"$'\n'"${BODY}"
  if ! printf '%s\n' "$FULL_MSG" | grep -q 'Ratchet-exception:.*\.machine_readable/root-allow\.txt'; then
    printf 'git-push-signed: ERROR: .machine_readable/root-allow.txt changed relative to origin/main.\n' >&2
    printf '  Include trailer in commit body: Ratchet-exception: .machine_readable/root-allow.txt -- <reason>\n' >&2
    exit 1
  fi
fi

REPO_NWO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"

# Seed remote branch at origin/main if it does not exist on origin yet.
if ! git rev-parse --verify "origin/$BRANCH" >/dev/null 2>&1; then
  MAIN_OID="$(git rev-parse origin/main)"
  gh api -X POST "repos/$REPO_NWO/git/refs" \
    -f "ref=refs/heads/$BRANCH" \
    -f "sha=$MAIN_OID" >/dev/null
  git fetch origin "$BRANCH"
fi

EXPECTED_OID="$(git rev-parse "origin/$BRANCH")"

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# Record remote git tree modes + git blob hashes so untouched symlinks (120000),
# submodules (160000), and unchanged blobs are detected without child spawns.
git ls-tree -r "origin/$BRANCH" > "$WORK_DIR/remote_tree.tsv"

# Restrict candidate paths strictly to staged and tracked-modified files.
git ls-files --cached --modified > "$WORK_DIR/candidates.txt"

# Precompute local git blob hashes for regular candidate files via stdin batching.
: > "$WORK_DIR/local_blobs.tsv"
while IFS= read -r rel; do
  [ -n "$rel" ] || continue
  if [ -f "$rel" ] && [ ! -L "$rel" ]; then
    blob_sha="$(git hash-object "$rel")"
    printf '%s\t%s\n' "$blob_sha" "$rel" >> "$WORK_DIR/local_blobs.tsv"
  fi
done < <(sort -u "$WORK_DIR/candidates.txt")

REPO_NWO="$REPO_NWO" \
BRANCH="$BRANCH" \
EXPECTED_OID="$EXPECTED_OID" \
HEADLINE="$HEADLINE" \
BODY="$BODY" \
WORK_DIR="$WORK_DIR" \
bun -e '
  import fs from "node:fs";
  import path from "node:path";

  const workDir = process.env.WORK_DIR;
  const repoNwo = process.env.REPO_NWO;
  const branch = process.env.BRANCH;
  const expectedOid = process.env.EXPECTED_OID;
  const headline = process.env.HEADLINE;
  const body = process.env.BODY || "";

  const remoteEntries = new Map();
  const treeLines = fs
    .readFileSync(path.join(workDir, "remote_tree.tsv"), "utf8")
    .split("\n");
  for (const line of treeLines) {
    if (!line) continue;
    const tab = line.indexOf("\t");
    const meta = line.slice(0, tab).split(/\s+/);
    remoteEntries.set(line.slice(tab + 1), { mode: meta[0], sha: meta[2] });
  }

  const localHashes = new Map();
  const hashLines = fs
    .readFileSync(path.join(workDir, "local_blobs.tsv"), "utf8")
    .split("\n");
  for (const line of hashLines) {
    if (!line) continue;
    const tab = line.indexOf("\t");
    localHashes.set(line.slice(tab + 1), line.slice(0, tab));
  }

  const additions = [];
  const deletions = [];

  for (const [rel, localSha] of Array.from(localHashes.entries()).sort((a, b) =>
    a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0,
  )) {
    const remote = remoteEntries.get(rel);
    if (remote && remote.sha === localSha) {
      continue;
    }
    additions.push({
      path: rel,
      contents: fs.readFileSync(rel).toString("base64"),
    });
  }

  for (const [rel, meta] of Array.from(remoteEntries.entries()).sort((a, b) =>
    a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0,
  )) {
    if (meta.mode !== "100644" && meta.mode !== "100755") {
      try {
        fs.lstatSync(rel);
        continue;
      } catch {
        deletions.push({ path: rel });
        continue;
      }
    }
    let existsFile = false;
    try {
      existsFile = fs.lstatSync(rel).isFile();
    } catch {
      existsFile = false;
    }
    if (!existsFile) {
      deletions.push({ path: rel });
    }
  }

  if (additions.length === 0 && deletions.length === 0) {
    process.exit(0);
  }

  const query = `
    mutation($input: CreateCommitOnBranchInput!) {
      createCommitOnBranch(input: $input) {
        commit {
          oid
          url
          signature { isValid state signer { login } }
        }
      }
    }
  `;

  const message = body ? { headline, body } : { headline };
  fs.writeFileSync(
    path.join(workDir, "payload.json"),
    JSON.stringify({
      query,
      variables: {
        input: {
          branch: { repositoryNameWithOwner: repoNwo, branchName: branch },
          expectedHeadOid: expectedOid,
          message,
          fileChanges: { additions, deletions },
        },
      },
    }),
    "utf8",
  );
'

if [ ! -f "$WORK_DIR/payload.json" ]; then
  printf 'git-push-signed: tracked/staged files match origin/%s; nothing to commit.\n' "$BRANCH" >&2
  exit 0
fi

gh api graphql --input "$WORK_DIR/payload.json" > "$WORK_DIR/response.json"

COMMIT_JSON="$(
  WORK_DIR="$WORK_DIR" bun -e '
    import fs from "node:fs";
    import path from "node:path";

    const raw = fs.readFileSync(path.join(process.env.WORK_DIR, "response.json"), "utf8");
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch (err) {
      process.stderr.write(`git-push-signed: invalid GraphQL JSON response: ${err}\n`);
      process.exit(1);
    }
    const commit = parsed?.data?.createCommitOnBranch?.commit;
    if (!commit?.signature?.isValid) {
      process.stderr.write(`git-push-signed: commit signature invalid: ${raw}\n`);
      process.exit(1);
    }
    process.stdout.write(JSON.stringify(commit) + "\n");
  '
)"

printf '[git-push-signed] Signed commit pushed: %s\n' "$COMMIT_JSON"

HAS_STASH=0
if [ -n "$(git status --porcelain)" ]; then
  git stash push --include-untracked -q -m "git-push-signed-sync"
  HAS_STASH=1
fi
git pull --ff-only origin "$BRANCH"
if [ "$HAS_STASH" -eq 1 ]; then
  git stash drop -q
fi
chmod +x .githooks/install.sh .githooks/pre-commit .githooks/commit-msg .githooks/pre-push \
         scripts/preflight.sh scripts/git-push-signed.sh scripts/sync-actions-lock.js 2>/dev/null || true
printf '[git-push-signed] Local branch %s synced cleanly at %s\n' "$BRANCH" "$(git rev-parse --short HEAD)"
