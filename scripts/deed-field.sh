#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# deed-field.sh — read one field out of a deed (<repo>_chora.deed, or any
# *_praxis.deed: the root is the first form with an ABNF doc-head).
#
# Usage:
#   deed-field.sh DEED PATH KEY           first value of :KEY in the clause at PATH
#   deed-field.sh --list DEED PATH KEY    the items of the list :KEY ( … ), one per line
#   deed-field.sh --clause DEED PATH      the clause, one token per line (for hashing)
#   deed-field.sh --has DEED PATH         exit 0 iff the clause exists
#   deed-field.sh --find [DIR]            print the one *_chora.deed in DIR (default .)
#   deed-field.sh --uuid DEED             the repo uuid, derived from :repo-uuid (bun > uuidgen)
set -euo pipefail

mode=value
case "${1:-}" in
  --list|--clause|--has|--uuid) mode="${1#--}"; shift ;;
  --find)
    shopt -s nullglob
    deeds=("${2:-.}"/*_chora.deed)
    [ "${#deeds[@]}" -eq 1 ] || exit 1
    printf '%s\n' "${deeds[0]}"
    exit 0 ;;
esac

deed="${1:-}"; path="${2-}"; key="${3:-}"
[ "$mode" != uuid ] || { path=""; key=repo-uuid; }
if [ -z "$deed" ] || [ ! -r "$deed" ]; then
  echo "deed-field.sh: no readable deed: '${deed}'" >&2; exit 2
fi
if { [ "$mode" = value ] || [ "$mode" = list ]; } && [ -z "$key" ]; then
  echo "deed-field.sh: KEY is required" >&2; exit 2
fi

read_deed() {
awk -v mode="$1" -v path="$path" -v key="$key" '
BEGIN { RS = "\001" }
{
  s = $0; n = length(s); i = 1; nt = 0
  while (i <= n) {
    c = substr(s, i, 1)
    if (c == ";") { while (i <= n && substr(s, i, 1) != "\n") i++; continue }
    if (c == "(" || c == ")") { tok[++nt] = c; i++; continue }
    if (c == "\"") {
      v = ""; i++
      while (i <= n) {
        c = substr(s, i, 1)
        if (c == "\\") {
          d = substr(s, i + 1, 1)
          if (d == "n") v = v "\n"; else if (d == "t") v = v "\t"; else v = v d
          i += 2; continue
        }
        if (c == "\"") break
        v = v c; i++
      }
      i++; tok[++nt] = "S" v; continue
    }
    if (c == " " || c == "\t" || c == "\r" || c == "\n") { i++; continue }
    v = ""
    while (i <= n) {
      c = substr(s, i, 1)
      if (c == " " || c == "\t" || c == "\r" || c == "\n" || c == "(" || c == ")" || c == "\"" || c == ";") break
      v = v c; i++
    }
    tok[++nt] = "A" v
  }
}
function close_of(o,   d, t) {
  d = 0
  for (t = o; t <= nt; t++) {
    if (tok[t] == "(") d++
    else if (tok[t] == ")") { d--; if (d == 0) return t }
  }
  return 0
}
function child(o, name,   e, t, d) {
  e = close_of(o); d = 0
  for (t = o + 1; t < e; t++) {
    if (tok[t] == "(") { if (d == 0 && tok[t + 1] == "A" name) return t; d++ }
    else if (tok[t] == ")") d--
  }
  return 0
}
function show(x) {
  x = substr(x, 2)
  if (x == "#t") return "true"
  if (x == "#f") return "false"
  return x
}
END {
  o = 0
  for (t = 1; t <= nt; t++)
    if (tok[t] == "(" && tok[t + 1] ~ /^A(repo-deed|praxis-deed|estate-deed|estate-atlas-deed)$/) { o = t; break }
  if (!o) exit 1
  if (path != "") {
    np = split(path, part, "/")
    for (p = 1; p <= np; p++) { o = child(o, part[p]); if (!o) exit 1 }
  }
  e = close_of(o)
  if (!e) exit 2
  if (mode == "has") exit 0
  if (mode == "clause") { for (t = o; t <= e; t++) print tok[t]; exit 0 }
  d = 0
  for (t = o + 1; t < e; t++) {
    if (tok[t] == "(") { d++; continue }
    if (tok[t] == ")") { d--; continue }
    if (d != 0 || tok[t] != "A:" key) continue
    nx = tok[t + 1]
    if (mode == "value") {
      if (nx == "(" || nx == ")" || nx == "") exit 1
      if (nx ~ /^A#[a-z0-9]+$/ && substr(tok[t + 2], 1, 1) == "S") nx = tok[t + 2]
      print show(nx); exit 0
    }
    if (nx != "(") exit 1
    le = close_of(t + 1); ld = 0
    for (u = t + 2; u < le; u++) {
      if (tok[u] == "(") ld++
      else if (tok[u] == ")") ld--
      else if (ld == 0) print show(tok[u])
    }
    exit 0
  }
  exit 1
}
' "$deed"
}

# uuid5_url NAME -> RFC 9562 URL-namespace UUIDv5 (Bun preferred first per estate policy)
uuid5_url() {
  if command -v bun >/dev/null 2>&1; then
    bun -e '
      const h = new Bun.CryptoHasher("sha1");
      h.update(Buffer.from("6ba7b8119dad11d180b400c04fd430c8", "hex"));
      h.update(process.argv[1]);
      const b = h.digest().subarray(0, 16);
      b[6] = (b[6] & 0x0f) | 0x50;
      b[8] = (b[8] & 0x3f) | 0x80;
      const x = b.toString("hex");
      console.log([x.slice(0, 8), x.slice(8, 12), x.slice(12, 16), x.slice(16, 20), x.slice(20)].join("-"));
    ' "$1"
  elif uuidgen --sha1 --namespace @url --name x >/dev/null 2>&1; then
    uuidgen --sha1 --namespace @url --name "$1"
  else
    echo "deed-field.sh: --uuid needs bun or uuidgen with --sha1" >&2; return 2
  fi
}

if [ "$mode" = uuid ]; then
  name="$(read_deed value)" || exit $?
  id="$(uuid5_url "$name")" || exit 2
  [[ "$id" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-5[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]] \
    || { echo "deed-field.sh: derived uuid is not uuid5-shaped: '$id'" >&2; exit 2; }
  printf '%s\n' "$id"
  exit 0
fi
read_deed "$mode"
