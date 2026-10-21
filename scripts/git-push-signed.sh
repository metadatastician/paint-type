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
# Usage:
#   ./scripts/git-push-signed.sh "feat(scope): short summary" ["optional body / trailers"]

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
export PATH="$HOME/.bun/bin:/tmp/tools-bin:/usr/local/bin:$PATH"

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
if ! git diff --quiet origin/main -- .machine_readable/root-allow.txt 2>/dev/null; then
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

COMMIT_JSON="$(
  REPO_NWO="$REPO_NWO" \
  BRANCH="$BRANCH" \
  EXPECTED_OID="$EXPECTED_OID" \
  HEADLINE="$HEADLINE" \
  BODY="$BODY" \
  bun -e '
    import fs from "node:fs";
    import { execFileSync, spawnSync } from "node:child_process";

    const repoNwo = process.env.REPO_NWO;
    const branch = process.env.BRANCH;
    const expectedOid = process.env.EXPECTED_OID;
    const headline = process.env.HEADLINE;
    const body = process.env.BODY || "";

    // Record regular files vs symlinks in origin/<branch> so untouched symlinks
    // (mode 120000) and submodules (mode 160000) are preserved intact.
    const remoteEntries = new Map();
    const lsTree = execFileSync("git", ["ls-tree", "-r", `origin/${branch}`], {
      encoding: "utf8",
    });
    for (const line of lsTree.split("\n")) {
      if (!line) continue;
      const tab = line.indexOf("\t");
      const meta = line.slice(0, tab).split(/\s+/);
      remoteEntries.set(line.slice(tab + 1), { mode: meta[0], sha: meta[2] });
    }

    const candidatePaths = new Set(
      execFileSync(
        "git",
        ["ls-files", "--cached", "--others", "--exclude-standard"],
        { encoding: "utf8" },
      )
        .split("\n")
        .filter(Boolean),
    );

    const additions = [];
    const deletions = [];

    for (const rel of Array.from(candidatePaths).sort()) {
      let st = null;
      try {
        st = fs.lstatSync(rel);
      } catch {
        st = null;
      }
      if (!st) continue;
      // Preserve symlinks / directory entries untouched in parent tree.
      if (!st.isFile()) continue;

      const localBytes = fs.readFileSync(rel);
      let same = false;
      if (remoteEntries.has(rel)) {
        const remoteRes = spawnSync("git", ["show", `origin/${branch}:${rel}`]);
        if (remoteRes.status === 0 && Buffer.compare(localBytes, remoteRes.stdout) === 0) {
          same = true;
        }
      }
      if (!same) {
        additions.push({
          path: rel,
          contents: localBytes.toString("base64"),
        });
      }
    }

    for (const [rel, meta] of Array.from(remoteEntries.entries()).sort((a, b) =>
      a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0,
    )) {
      if (meta.mode !== "100644" && meta.mode !== "100755") {
        // Leave tracked symlinks / gitlinks untouched unless explicitly removed from disk.
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
      process.stderr.write(`git-push-signed: working tree matches origin/${branch}; nothing to commit.\n`);
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
    const payload = JSON.stringify({
      query,
      variables: {
        input: {
          branch: { repositoryNameWithOwner: repoNwo, branchName: branch },
          expectedHeadOid: expectedOid,
          message,
          fileChanges: { additions, deletions },
        },
      },
    });

    const ghRes = spawnSync("gh", ["api", "graphql", "--input", "-"], {
      input: payload,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    });
    if (ghRes.status !== 0) {
      process.stderr.write(ghRes.stderr || ghRes.stdout || "gh api graphql failed\n");
      process.exit(1);
    }
    const parsed = JSON.parse(ghRes.stdout);
    const commit = parsed?.data?.createCommitOnBranch?.commit;
    if (!commit?.signature?.isValid) {
      process.stderr.write(`git-push-signed: commit signature invalid: ${ghRes.stdout}\n`);
      process.exit(1);
    }
    process.stdout.write(JSON.stringify(commit) + "\n");
  '
)"

[ -n "$COMMIT_JSON" ] || exit 0
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
