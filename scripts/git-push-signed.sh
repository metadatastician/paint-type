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
# Notes:
#   - Stage newly created files explicitly with `git add <file>` first; this
#     script inspects only tracked/staged paths (`git ls-files -z --cached --modified`)
#     so untracked local scratch artifacts are never published automatically.
#   - `createCommitOnBranch` preserves existing tree file modes on updates and
#     materializes brand-new files as `100644`. If a local file's executable bit
#     differs from `origin/<branch>`, this script aborts unless
#     `ALLOW_MODE_FALLBACK=1` is set explicitly.
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

# Use NUL-delimited output (`-z`) so non-ASCII or special filenames are never
# C-quoted or corrupted.
git ls-tree -r -z "origin/$BRANCH" > "$WORK_DIR/remote_tree.bin"
git ls-files -z --cached --modified > "$WORK_DIR/candidates.bin"

# Materialize clean-filtered git blobs (`git hash-object -w`) so CRLF / filter
# rules never cause false diffs or upload smudged working-tree bytes.
: > "$WORK_DIR/local_blobs.bin"
declare -A SEEN_CANDIDATES=()
while IFS= read -r -d '' rel; do
  [ -n "$rel" ] || continue
  if [ -n "${SEEN_CANDIDATES[$rel]:-}" ]; then
    continue
  fi
  SEEN_CANDIDATES["$rel"]=1
  if [ -f "$rel" ] && [ ! -L "$rel" ]; then
    blob_sha="$(git hash-object -w -- "$rel")"
    b64_file="$WORK_DIR/blob_${blob_sha}.b64"
    if [ ! -f "$b64_file" ]; then
      git cat-file blob "$blob_sha" | base64 | tr -d '\r\n' > "$b64_file"
    fi
    if [ -x "$rel" ]; then
      local_mode="100755"
    else
      local_mode="100644"
    fi
    printf '%s\t%s\t%s\0' "$local_mode" "$blob_sha" "$rel" >> "$WORK_DIR/local_blobs.bin"
  fi
done < "$WORK_DIR/candidates.bin"

REPO_NWO="$REPO_NWO" \
BRANCH="$BRANCH" \
EXPECTED_OID="$EXPECTED_OID" \
HEADLINE="$HEADLINE" \
BODY="$BODY" \
WORK_DIR="$WORK_DIR" \
ALLOW_MODE_FALLBACK="${ALLOW_MODE_FALLBACK:-0}" \
bun -e '
  import fs from "node:fs";
  import path from "node:path";

  const workDir = process.env.WORK_DIR;
  const repoNwo = process.env.REPO_NWO;
  const branch = process.env.BRANCH;
  const expectedOid = process.env.EXPECTED_OID;
  const headline = process.env.HEADLINE;
  const body = process.env.BODY || "";
  const allowModeFallback = process.env.ALLOW_MODE_FALLBACK === "1";

  const remoteEntries = new Map();
  const treeRecords = fs
    .readFileSync(path.join(workDir, "remote_tree.bin"), "utf8")
    .split("\0");
  for (const rec of treeRecords) {
    if (!rec) continue;
    const tab = rec.indexOf("\t");
    const meta = rec.slice(0, tab).split(/\s+/);
    remoteEntries.set(rec.slice(tab + 1), { mode: meta[0], sha: meta[2] });
  }

  const localEntries = new Map();
  const localRecords = fs
    .readFileSync(path.join(workDir, "local_blobs.bin"), "utf8")
    .split("\0");
  for (const rec of localRecords) {
    if (!rec) continue;
    const firstTab = rec.indexOf("\t");
    const secondTab = rec.indexOf("\t", firstTab + 1);
    const mode = rec.slice(0, firstTab);
    const sha = rec.slice(firstTab + 1, secondTab);
    const rel = rec.slice(secondTab + 1);
    localEntries.set(rel, { mode, sha });
  }

  const additions = [];
  const deletions = [];
  const modeProblems = [];

  for (const [rel, local] of Array.from(localEntries.entries()).sort((a, b) =>
    a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0,
  )) {
    const remote = remoteEntries.get(rel);
    const expectedRemoteMode = remote ? remote.mode : "100644";
    if (local.mode !== expectedRemoteMode) {
      modeProblems.push(`${rel} (local ${local.mode} vs target ${expectedRemoteMode})`);
    }
    if (remote && remote.sha === local.sha) {
      continue;
    }
    const b64 = fs.readFileSync(path.join(workDir, `blob_${local.sha}.b64`), "utf8");
    additions.push({ path: rel, contents: b64 });
  }

  if (modeProblems.length > 0 && !allowModeFallback) {
    process.stderr.write(
      "git-push-signed: ERROR: createCommitOnBranch cannot alter Git executable file modes:\n  " +
        modeProblems.join("\n  ") +
        "\nUse a signed local Git push to set 100755 modes, or set ALLOW_MODE_FALLBACK=1 if 100644 is intended.\n",
    );
    process.exit(1);
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
  trap 'rm -rf "$WORK_DIR"; printf "git-push-signed: local changes preserved in stash@{0} (git stash pop)\n" >&2' ERR
fi
git fetch origin "$BRANCH"
git reset -q --keep "origin/$BRANCH"
if [ "$HAS_STASH" -eq 1 ]; then
  if git diff --quiet "stash@{0}" \
    && [ -z "$(git ls-files --others --exclude-standard)" ] \
    && [ -z "$(git show --name-only --format= 'stash@{0}^3' 2>/dev/null)" ]; then
    git stash drop -q
  else
    git stash pop -q || printf 'git-push-signed: WARNING: unpushed local edits kept in stash@{0}; resolve with git stash pop\n' >&2
  fi
  trap 'rm -rf "$WORK_DIR"' EXIT
  trap - ERR
fi
printf '[git-push-signed] Local branch %s synced cleanly at %s\n' "$BRANCH" "$(git rev-parse --short HEAD)"
