#!/usr/bin/env bun
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// validate-coordination-k9.js — deterministic validator for `coordination.k9`
// against hyperpolymath/standards:
//   2-protocols/k9-coordination/spec/abnf/coordination-k9-grammar_v1.2.abnf
//
// Closes the validation gap documented in hyperpolymath/standards
// docs/CONTRACTILE-DESCRIPTILE-VALIDATION.adoc: `k9-validate.sh` recognizes
// the `coordination` dialect at L0 and routes it (`verdict: "routed"`), so
// consumer repositories must validate `coordination.k9` directly against the
// v1.2.0 ABNF grammar.

import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const CANONICAL_SECTION_ORDER = [
  "metadata",
  "project",
  "build_commands",
  "invariants",
  "protected",
  "architecture",
  "innervation",
  "do_not_create",
  "terminology",
  "ecosystem",
  "coordination",
];

const REQUIRED_SECTIONS = [
  "metadata",
  "project",
  "build_commands",
  "invariants",
  "protected",
  "architecture",
  "do_not_create",
];

const SEVERITIES = new Set(["critical", "high", "medium", "low"]);
const GENERATED_BY = new Set(["manual", "generate-coordination-k9"]);
const REFLEX_TRIGGERS = new Set(["pre-commit", "file-create", "pull-request", "post-merge"]);

export function validateCoordinationK9(text, filePath = "coordination.k9") {
  const errors = [];
  const fail = (lineNo, code, msg) =>
    errors.push({ file: filePath, line: lineNo, code, message: msg });

  if (text.includes("\0")) {
    fail(1, "CK9-001", "file contains NUL byte");
    return errors;
  }
  if (text.includes("\r")) {
    fail(1, "CK9-002", "CRLF detected; grammar mandates LF (%x0A) line endings");
  }

  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();

  // magic-line = "K9!" LF
  if (lines[0] !== "K9!") {
    fail(1, "CK9-003", `line 1 must be exact magic 'K9!', got '${lines[0] ?? ""}'`);
  }

  // spdx-line = "#" 1*WSP "SPDX-License-Identifier:" 1*WSP value LF
  if (!/^#[ \t]+SPDX-License-Identifier:[ \t]+\S+/.test(lines[1] ?? "")) {
    fail(2, "CK9-004", "line 2 must carry '# SPDX-License-Identifier: <expr>'");
  }

  const sections = new Map();
  let currentSection = null;
  let currentSub = null;
  let currentItem = null;

  for (let idx = 2; idx < lines.length; idx++) {
    const lineNo = idx + 1;
    const line = lines[idx];

    if (line === "") continue;

    // Ruling D318: YAML document separator `---` is forbidden in coordination.k9
    if (/^---\s*$/.test(line)) {
      fail(lineNo, "CK9-005", "YAML document separator '---' is invalid in coordination.k9 (ruling D318)");
      continue;
    }

    // Comment lines: allow printable UTF-8, reject control characters
    if (/^\s*#/.test(line)) {
      if (/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/.test(line)) {
        fail(lineNo, "CK9-006", "comment line contains forbidden control character");
      }
      continue;
    }

    if (line.includes("\t")) {
      fail(lineNo, "CK9-007", "indentation must use spaces (2-space steps), not tabs");
      continue;
    }

    const indent = line.match(/^ */)[0].length;
    if (indent % 2 !== 0 || indent > 8) {
      fail(lineNo, "CK9-008", `invalid indentation depth ${indent} (expected 0, 2, 4, 6, or 8 spaces)`);
      continue;
    }

    if (indent === 0) {
      const m = line.match(/^([a-z_]+):$/);
      if (!m) {
        fail(lineNo, "CK9-009", `invalid top-level section header '${line}'`);
        continue;
      }
      const name = m[1];
      if (!CANONICAL_SECTION_ORDER.includes(name)) {
        fail(lineNo, "CK9-010", `unknown top-level section '${name}'`);
        continue;
      }
      if (sections.has(name)) {
        fail(lineNo, "CK9-011", `duplicate top-level section '${name}'`);
        continue;
      }
      sections.set(name, { line: lineNo, scalars: new Map(), lists: new Map(), items: [], subs: new Map() });
      currentSection = sections.get(name);
      currentSub = null;
      currentItem = null;
      continue;
    }

    if (!currentSection) {
      fail(lineNo, "CK9-012", "indented content before any top-level section");
      continue;
    }

    const trimmed = line.slice(indent);
    if (indent === 2) {
      if (trimmed.startsWith("- ")) {
        currentItem = { line: lineNo, fields: new Map(), raw: trimmed.slice(2) };
        const kv = trimmed.slice(2).match(/^([a-z0-9_-]+):\s+(.+)$/);
        if (kv) currentItem.fields.set(kv[1], kv[2]);
        currentSection.items.push(currentItem);
        currentSub = null;
      } else {
        const sub = trimmed.match(/^([a-z0-9_-]+):$/);
        const kv = trimmed.match(/^([a-z0-9_-]+):\s+(.+)$/);
        if (sub) {
          currentSub = sub[1];
          currentSection.subs.set(currentSub, []);
          currentSection.lists.set(currentSub, []);
          currentItem = null;
        } else if (kv) {
          currentSection.scalars.set(kv[1], kv[2]);
          currentSub = null;
          currentItem = null;
        } else {
          fail(lineNo, "CK9-013", `malformed level-1 entry '${trimmed}'`);
        }
      }
    } else if (indent === 4) {
      if (trimmed.startsWith("- ")) {
        const rest = trimmed.slice(2);
        const kv = rest.match(/^([a-z0-9_-]+):\s+(.+)$/);
        if (kv && currentSub && currentSection.subs.has(currentSub)) {
          currentItem = { line: lineNo, fields: new Map([[kv[1], kv[2]]]) };
          currentSection.subs.get(currentSub).push(currentItem);
        } else if (currentSub && currentSection.lists.has(currentSub)) {
          currentSection.lists.get(currentSub).push(rest);
        } else if (currentItem) {
          currentItem.fields.set("_item", rest);
        }
      } else if (currentItem) {
        const kv = trimmed.match(/^([a-z0-9_-]+):\s*(.*)$/);
        if (kv) {
          currentItem.fields.set(kv[1], kv[2]);
        } else {
          fail(lineNo, "CK9-014", `malformed item attribute '${trimmed}'`);
        }
      }
    } else if (indent === 6 || indent === 8) {
      if (!trimmed.startsWith("- ") && !/^[a-z0-9_-]+:\s*.*$/.test(trimmed)) {
        fail(lineNo, "CK9-015", `malformed nested entry '${trimmed}'`);
      }
    }
  }

  // Enforce canonical section order from coordination-k9-grammar_v1.2.abnf
  const seenOrder = [...sections.keys()];
  let prevIdx = -1;
  for (const name of seenOrder) {
    const pos = CANONICAL_SECTION_ORDER.indexOf(name);
    if (pos < prevIdx) {
      fail(
        sections.get(name).line,
        "CK9-016",
        `section '${name}' appears out of canonical ABNF order (${CANONICAL_SECTION_ORDER.join(" -> ")})`,
      );
    }
    prevIdx = pos;
  }

  for (const req of REQUIRED_SECTIONS) {
    if (!sections.has(req)) {
      fail(1, "CK9-017", `missing required section '${req}:'`);
    }
  }

  // Semantic & value production checks per section
  const meta = sections.get("metadata");
  if (meta) {
    if (meta.scalars.get("schema") !== "k9-coordination") {
      fail(meta.line, "CK9-018", `metadata.schema must be 'k9-coordination'`);
    }
    if (!/^1\.[12]\.\d+$/.test(meta.scalars.get("schema_version") ?? "")) {
      fail(meta.line, "CK9-019", `metadata.schema_version must be 1.1.x or 1.2.x`);
    }
    if (!GENERATED_BY.has(meta.scalars.get("generated_by") ?? "")) {
      fail(meta.line, "CK9-020", `metadata.generated_by must be 'manual' or 'generate-coordination-k9'`);
    }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(meta.scalars.get("last_updated") ?? "")) {
      fail(meta.line, "CK9-021", `metadata.last_updated must match YYYY-MM-DD`);
    }
  }

  const proj = sections.get("project");
  if (proj) {
    for (const k of ["name", "description", "license", "build_system"]) {
      if (!proj.scalars.get(k)) fail(proj.line, "CK9-022", `project.${k} is required`);
    }
    if ((proj.lists.get("languages") ?? []).length === 0) {
      fail(proj.line, "CK9-023", "project.languages must list at least one language");
    }
    const deedRef = proj.scalars.get("deed_ref");
    if (deedRef) {
      const target = resolve(dirname(resolve(filePath)), deedRef);
      if (!existsSync(target)) {
        fail(proj.line, "CK9-024", `project.deed_ref '${deedRef}' does not exist at ${target}`);
      }
    }
  }

  for (const [secName, minCount, requiredFields] of [
    ["build_commands", 1, ["command", "purpose"]],
    ["invariants", 1, ["id", "rule", "reason", "severity"]],
    ["protected", 1, ["path", "reason"]],
    ["architecture", 1, ["id", "decision", "reason"]],
    ["do_not_create", 1, ["pattern", "reason"]],
  ]) {
    const sec = sections.get(secName);
    if (!sec) continue;
    if (sec.items.length < minCount) {
      fail(sec.line, "CK9-025", `section '${secName}' must declare at least ${minCount} item`);
    }
    for (const item of sec.items) {
      for (const f of requiredFields) {
        if (!item.fields.get(f)) {
          fail(item.line, "CK9-026", `${secName} item missing required field '${f}'`);
        }
      }
      if (secName === "invariants" && item.fields.get("severity") && !SEVERITIES.has(item.fields.get("severity"))) {
        fail(item.line, "CK9-027", `invalid invariant severity '${item.fields.get("severity")}'`);
      }
    }
  }

  const inn = sections.get("innervation");
  if (inn) {
    for (const item of inn.subs.get("reflex") ?? []) {
      const trig = (item.fields.get("trigger") ?? "").replace(/^"|"$/g, "");
      if (!REFLEX_TRIGGERS.has(trig)) {
        fail(item.line, "CK9-028", `innervation.reflex trigger '${trig}' must be one of ${[...REFLEX_TRIGGERS].join(", ")}`);
      }
    }
  }

  return errors;
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const jsonMode = args[0] === "--json";
  const target = jsonMode ? args[1] : args[0];
  if (!target) {
    console.error("usage: bun scripts/validate-coordination-k9.js [--json] <coordination.k9>");
    process.exit(2);
  }
  const text = readFileSync(target, "utf8");
  const errors = validateCoordinationK9(text, target);
  if (jsonMode) {
    console.log(
      JSON.stringify({
        file: target,
        grammar: "coordination-k9-grammar_v1.2.abnf",
        verdict: errors.length === 0 ? "pass" : "fail",
        errors,
      }),
    );
  } else if (errors.length === 0) {
    console.log(`PASS  ${target} (coordination-k9-grammar_v1.2.abnf)`);
  } else {
    for (const e of errors) {
      console.error(`ERROR [${e.code}] ${e.file}:${e.line}: ${e.message}`);
    }
  }
  process.exit(errors.length === 0 ? 0 : 1);
}
