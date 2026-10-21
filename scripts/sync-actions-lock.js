#!/usr/bin/env bun
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Deterministic synchronizer, verifier, and conflict-healer for
// `.github/workflows/actions.lock` (portable across Linux, macOS, and Windows
// without requiring GNU `gawk` or shell subprocesses).
//
// Usage:
//   bun scripts/sync-actions-lock.js           # Rewrite actions.lock in canonical LC_ALL=C order
//   bun scripts/sync-actions-lock.js --write   # Same as default
//   bun scripts/sync-actions-lock.js --verify  # Enforce all 4 lock-sync-gate invariants read-only
//   bun scripts/sync-actions-lock.js --check   # Enforce --verify AND exact canonical byte order

import fs from "node:fs";
import path from "node:path";

/**
 * Parse a JSON payload with explicit error context rather than throwing an
 * opaque syntax exception at an input boundary.
 * @param {string} raw Raw JSON string from an API response.
 * @param {string} context Human-readable description of the source endpoint.
 * @returns {any} Parsed value.
 */
function parseJsonSafe(raw, context) {
  try {
    return JSON.parse(raw);
  } catch (err) {
    const reason = err instanceof Error ? err.message : String(err);
    throw new Error(`Invalid JSON payload from ${context}: ${reason}`);
  }
}

/**
 * Perform an authenticated GET against `api.github.com` without spawning any
 * external process.
 * @param {string} apiPath Relative REST endpoint under `https://api.github.com/`.
 * @returns {Promise<any>} Parsed JSON response body.
 */
async function fetchGitHubJson(apiPath) {
  const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN || "";
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "paint-type-sync-actions-lock",
  };
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  const url = `https://api.github.com/${apiPath}`;
  const resp = await fetch(url, { headers });
  const text = await resp.text();
  if (!resp.ok) {
    throw new Error(`GitHub API ${resp.status} for ${apiPath}: ${text.slice(0, 200)}`);
  }
  return parseJsonSafe(text, apiPath);
}

/**
 * Compare two strings in strict ASCII / POSIX `LC_ALL=C` byte order.
 * @param {string} a First string.
 * @param {string} b Second string.
 * @returns {number} Negative if a < b, positive if a > b, 0 if equal.
 */
function cmpAscii(a, b) {
  if (a === b) return 0;
  return a < b ? -1 : 1;
}

/**
 * Strip standard git 3-way conflict markers while retaining lines from both
 * sides so dependency metadata blocks survive conflict healing.
 * @param {string} raw Raw file content.
 * @returns {string} Marker-free text.
 */
function stripConflictMarkers(raw) {
  return raw
    .split(/\r?\n/)
    .filter(
      (line) =>
        !line.startsWith("<<<<<<<") &&
        !line.startsWith("=======") &&
        !line.startsWith(">>>>>>>") &&
        !line.startsWith("|||||||"),
    )
    .join("\n");
}

/**
 * Normalize an external `uses:` value into canonical `owner/repo@ref` form,
 * discarding action subpaths while ignoring local `./` or `$/` references.
 * @param {string} raw Raw `uses:` value from a workflow step or job.
 * @returns {string} Normalized `owner/repo@ref`, or `""` for local refs.
 */
function normalizeUses(raw) {
  const cleaned = raw.trim().replace(/^['"]|['"]$/g, "").trim();
  if (!cleaned || cleaned.startsWith("./") || cleaned.startsWith("$/")) {
    return "";
  }
  const atIdx = cleaned.lastIndexOf("@");
  if (atIdx <= 0 || atIdx === cleaned.length - 1) return "";
  const repoPath = cleaned.slice(0, atIdx);
  const ref = cleaned.slice(atIdx + 1);
  const parts = repoPath.split("/");
  if (parts.length < 2 || !parts[0] || !parts[1]) return "";
  return `${parts[0]}/${parts[1]}@${ref}`;
}

/**
 * Compute case-folded lookup key matching `scripts/check-lock-sync.sh`:
 * lowercase `owner/repo` with case-sensitive `@ref`.
 * @param {string} ref Normalized `owner/repo@ref`.
 * @returns {string} Canonical lookup key.
 */
function foldKey(ref) {
  const atIdx = ref.lastIndexOf("@");
  if (atIdx <= 0) return ref.toLowerCase();
  return `${ref.slice(0, atIdx).toLowerCase()}${ref.slice(atIdx)}`;
}

/**
 * Parse header, workflow mappings, and dependency records from `actions.lock`.
 * @param {string} lockText Raw text of `actions.lock`.
 * @param {boolean} healConflicts Whether to strip git conflict markers first.
 * @returns {{ headerLines: string[], wfMap: Map<string, string[]>, deps: Map<string, { displayKey: string, bodyLines: string[] }> }}
 */
function parseLock(lockText, healConflicts) {
  const source = healConflicts ? stripConflictMarkers(lockText) : lockText;
  const lines = source.split(/\r?\n/);
  const headerLines = [];
  const wfMap = new Map();
  const deps = new Map();

  let section = "header";
  let currentWf = null;
  let currentKey = null;
  let currentBody = [];

  const flushDep = () => {
    if (currentKey !== null && currentBody.length > 0) {
      const fk = foldKey(currentKey);
      const prev = deps.get(fk);
      if (!prev || currentBody.length > prev.bodyLines.length) {
        deps.set(fk, { displayKey: foldKey(currentKey), bodyLines: currentBody });
      }
    }
    currentKey = null;
    currentBody = [];
  };

  for (const line of lines) {
    if (/^workflows:\s*$/.test(line)) {
      flushDep();
      section = "workflows";
      continue;
    }
    if (/^dependencies:\s*$/.test(line)) {
      flushDep();
      section = "dependencies";
      continue;
    }
    if (section === "header") {
      if (line.trim() !== "" || headerLines.length > 0) {
        headerLines.push(line);
      }
      continue;
    }
    if (section === "workflows") {
      const emptyWf = line.match(/^    '([^']+)':\s*\[\]\s*$/);
      if (emptyWf) {
        currentWf = emptyWf[1];
        if (!wfMap.has(currentWf)) wfMap.set(currentWf, []);
        continue;
      }
      const openWf = line.match(/^    '([^']+)':\s*$/);
      if (openWf) {
        currentWf = openWf[1];
        if (!wfMap.has(currentWf)) wfMap.set(currentWf, []);
        continue;
      }
      const itemWf = line.match(/^        -\s*'([^']+)'\s*$/);
      if (itemWf && currentWf !== null) {
        wfMap.get(currentWf).push(itemWf[1]);
      }
      continue;
    }
    if (section === "dependencies") {
      const m = line.match(/^    '([^']+)':\s*$/);
      if (m) {
        flushDep();
        currentKey = m[1];
      } else if (currentKey !== null && line.startsWith("        ")) {
        currentBody.push(line);
      }
    }
  }
  flushDep();

  while (headerLines.length > 0 && headerLines[headerLines.length - 1] === "") {
    headerLines.pop();
  }
  if (headerLines.length === 0) {
    headerLines.push(
      "# This file is machine-generated by `gh actions-lock`.",
      "# Do not edit by hand; run `gh actions-lock` to update.",
      "# Docs: https://gh.io/actions-lockfile",
      "version: 'v0.0.3'",
    );
  }
  return { headerLines, wfMap, deps };
}

/**
 * Extract sorted external `uses:` references from a workflow YAML file using
 * the identical comment-stripping and normalization rules as `check-lock-sync.sh`.
 * @param {string} filePath Path to `.yml` / `.yaml` workflow file.
 * @returns {string[]} Sorted unique external `owner/repo@ref` pins.
 */
function extractWorkflowUses(filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const found = new Map();
  for (const rawLine of text.split(/\r?\n/)) {
    const noComment = rawLine.replace(/\s+#.*$/, "");
    const m = noComment.match(/^\s*-?\s*uses:\s*(.+)$/);
    if (!m) continue;
    const norm = normalizeUses(m[1]);
    if (!norm) continue;
    const folded = foldKey(norm);
    found.set(folded, folded);
  }
  return Array.from(found.values()).sort(cmpAscii);
}

/**
 * Verify all four invariants enforced by `scripts/check-lock-sync.sh`:
 *   1. Every external `uses:` in each workflow is locked under its path.
 *   2. Every lock entry under a workflow is still referenced by that workflow.
 *   3. Every workflow file has a lock key (including `[]` for zero-`uses:`).
 *   4. Every locked/nested ref resolves to a top-level `dependencies:` record.
 * @param {string} wfDir Path to `.github/workflows`.
 * @param {string} lockText Raw text of `actions.lock`.
 * @returns {boolean} True when all four clauses hold.
 */
function verifyLockSync(wfDir, lockText) {
  if (/^(<<<<<<<|=======|>>>>>>>)/m.test(lockText)) {
    process.stderr.write("FAIL actions.lock contains unresolved git merge conflict markers\n");
    return false;
  }
  const { wfMap, deps } = parseLock(lockText, false);
  const wfFiles = fs
    .readdirSync(wfDir)
    .filter((f) => f.endsWith(".yml") || f.endsWith(".yaml"))
    .sort(cmpAscii);

  let ok = true;
  const wantedRefs = new Set();

  for (const [wfKey, items] of wfMap.entries()) {
    for (const item of items) wantedRefs.add(foldKey(item));
  }
  for (const rec of deps.values()) {
    for (const bLine of rec.bodyLines) {
      const m = bLine.match(/^\s{12}-\s*'([^']+)'/);
      if (m) {
        const nested = normalizeUses(m[1]);
        if (nested) wantedRefs.add(foldKey(nested));
      }
    }
  }

  for (const file of wfFiles) {
    const key = `.github/workflows/${file}`;
    const actual = new Set(extractWorkflowUses(path.join(wfDir, file)));
    if (!wfMap.has(key)) {
      process.stderr.write(`FAIL ${key}: missing workflow entry in actions.lock\n`);
      ok = false;
      continue;
    }
    const locked = new Set(wfMap.get(key).map(foldKey));
    for (const ref of actual) {
      if (!locked.has(ref)) {
        process.stderr.write(`FAIL ${key}: missing lock entry ${ref}\n`);
        ok = false;
      }
    }
    for (const ref of locked) {
      if (!actual.has(ref)) {
        process.stderr.write(`FAIL ${key}: orphan lock entry ${ref}\n`);
        ok = false;
      }
    }
  }

  const liveKeys = new Set(wfFiles.map((f) => `.github/workflows/${f}`));
  for (const lockedKey of wfMap.keys()) {
    if (!liveKeys.has(lockedKey)) {
      process.stderr.write(`FAIL ${lockedKey}: lock entry for deleted workflow\n`);
      ok = false;
    }
  }

  for (const want of wantedRefs) {
    if (/^[^/]+\/[^/@]+@/.test(want) && !deps.has(want)) {
      process.stderr.write(`FAIL actions.lock: dangling edge without dependencies record: ${want}\n`);
      ok = false;
    }
  }

  return ok;
}

/**
 * Resolve metadata for an unlisted `owner/repo@ref` dependency via native HTTPS
 * requests against the GitHub REST API.
 * @param {string} spec Normalized `owner/repo@ref`.
 * @returns {Promise<{ displayKey: string, bodyLines: string[] }>} Resolved record.
 */
async function resolveMissingDependency(spec) {
  const atIdx = spec.lastIndexOf("@");
  const ownerRepo = spec.slice(0, atIdx);
  const ref = spec.slice(atIdx + 1);
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(ownerRepo)) {
    throw new Error(`Invalid repository slug in action spec: ${spec}`);
  }

  const repoMeta = await fetchGitHubJson(`repos/${ownerRepo}`);
  const ownerId = repoMeta.owner.id;
  const repoId = repoMeta.id;

  let commitSha = ref;
  let tagRef = ref;
  if (/^[0-9a-f]{40}$/i.test(ref)) {
    try {
      const tags = await fetchGitHubJson(`repos/${ownerRepo}/tags?per_page=100`);
      const matched = Array.isArray(tags)
        ? tags.find((t) => t?.commit?.sha?.toLowerCase() === ref.toLowerCase())
        : null;
      if (matched && matched.name) {
        tagRef = matched.name;
      } else if (repoMeta.default_branch) {
        tagRef = repoMeta.default_branch;
      }
    } catch {
      if (repoMeta.default_branch) {
        tagRef = repoMeta.default_branch;
      }
    }
  } else {
    const commitMeta = await fetchGitHubJson(
      `repos/${ownerRepo}/commits/${encodeURIComponent(ref)}`,
    );
    commitSha = commitMeta.sha;
  }

  return {
    displayKey: foldKey(spec),
    bodyLines: [
      `        ref: '${tagRef}'`,
      `        commit: 'sha1-${commitSha.toLowerCase()}'`,
      `        owner_id: ${ownerId}`,
      `        repo_id: ${repoId}`,
    ],
  };
}

/**
 * CLI entry point supporting `--verify`, `--check`, and `--write`.
 * @returns {Promise<void>}
 */
async function main() {
  const args = new Set(process.argv.slice(2));
  const verifyOnly = args.has("--verify");
  const checkOnly = args.has("--check");
  const wfDir = path.resolve(".github/workflows");
  const lockPath = path.join(wfDir, "actions.lock");

  const existingText = fs.existsSync(lockPath)
    ? fs.readFileSync(lockPath, "utf8")
    : "";

  if (verifyOnly) {
    if (!verifyLockSync(wfDir, existingText)) {
      process.exit(1);
    }
    process.stdout.write("actions.lock is in sync and transitively closed (verified via Bun).\n");
    return;
  }

  const { headerLines, deps } = parseLock(existingText, true);
  const wfFiles = fs
    .readdirSync(wfDir)
    .filter((f) => f.endsWith(".yml") || f.endsWith(".yaml"))
    .sort(cmpAscii);

  const workflowEntries = [];
  const requiredRefs = new Set();

  for (const file of wfFiles) {
    const relKey = `.github/workflows/${file}`;
    const refs = extractWorkflowUses(path.join(wfDir, file));
    workflowEntries.push({ relKey, refs });
    for (const r of refs) requiredRefs.add(r);
  }

  const queue = Array.from(requiredRefs);
  while (queue.length > 0) {
    const item = queue.shift();
    const fk = foldKey(item);
    if (!deps.has(fk)) {
      deps.set(fk, await resolveMissingDependency(item));
    }
    const rec = deps.get(fk);
    for (const line of rec.bodyLines) {
      const m = line.match(/^\s{12}-\s*'([^']+)'/);
      if (m) {
        const nested = normalizeUses(m[1]);
        if (nested && !deps.has(foldKey(nested))) {
          queue.push(nested);
        }
      }
    }
  }

  const out = [...headerLines, "workflows:"];
  for (const { relKey, refs } of workflowEntries) {
    if (refs.length === 0) {
      out.push(`    '${relKey}': []`);
    } else {
      out.push(`    '${relKey}':`);
      for (const r of refs) {
        out.push(`        - '${deps.get(foldKey(r)).displayKey}'`);
      }
    }
  }

  out.push("dependencies:");
  const sortedDeps = Array.from(deps.values()).sort((a, b) =>
    cmpAscii(a.displayKey, b.displayKey),
  );
  for (const rec of sortedDeps) {
    out.push(`    '${rec.displayKey}':`);
    for (const bLine of rec.bodyLines) {
      out.push(bLine);
    }
  }
  out.push("");

  const rendered = out.join("\n");
  if (checkOnly) {
    if (existingText !== rendered) {
      process.stderr.write(
        "sync-actions-lock: actions.lock differs from canonical output. Run `just sync-lock`.\n",
      );
      process.exit(1);
    }
    process.stdout.write("sync-actions-lock: actions.lock matches canonical ordering.\n");
    return;
  }

  fs.writeFileSync(lockPath, rendered, "utf8");
  if (!verifyLockSync(wfDir, rendered)) {
    process.exit(1);
  }
}

await main();
