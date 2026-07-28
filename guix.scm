; SPDX-License-Identifier: MPL-2.0
;; guix.scm — GNU Guix package definition for paint-type
;; Usage: guix shell -f guix.scm

(use-modules (guix packages)
             (guix build-system gnu)
             (guix licenses))

(package
  (name "paint-type")
  (version "0.1.0")
  (source #f)
  (build-system gnu-build-system)
  (synopsis "paint-type")
  (description "paint-type — part of the hyperpolymath ecosystem.")
  (home-page "https://github.com/metadatastician/paint-type")
  (license ((@@ (guix licenses) license) "PMPL-1.0-or-later"
             "https://github.com/hyperpolymath/palimpsest-license")))
