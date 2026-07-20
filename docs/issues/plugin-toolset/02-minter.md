# Plugin Minter for paint-type

**Status:** Open  
**Priority:** High  
**Labels:** enhancement, plugin-system, tooling, packaging  
**Depends on:** #00 (Overview Issue)  

## Description

Create a plugin minter tool for paint-type that handles the creation of plugin packages and artifacts. This includes versioning, packaging, and preparing plugins for distribution.

## Inspiration

Examine similar tools in:
- `boj-server/boj-server` minters
- `panll` packaging tools
- Other estate repositories with minter functionality

## Requirements

### Functionality
- [ ] Bump plugin version according to semantic versioning
- [ ] Generate plugin package manifests
- [ ] Create distribution artifacts (tarballs, zip files, etc.)
- [ ] Verify package integrity (checksums, signatures)
- [ ] Support for multiple distribution formats
- [ ] Generate registry metadata for plugin catalog
- [ ] Validate plugin before minting

### Integration
- [ ] Integrate with paint-type's build pipeline
- [ ] Hook into CI/CD for automated minting
- [ ] Provide CLI commands for manual operation

### Package Formats
- [ ] Source distribution (tar.gz)
- [ ] Pre-built binaries (where applicable)
- [ ] WASM modules (for web-based plugins)
- [ ] Container images (for runtime plugins)

## Deliverables

1. `tools/plugin-minter/` directory with implementation
2. CLI command: `paint-type plugin mint` or `plugin-minter`
3. Documentation in `docs/tools/plugin-minter.adoc`
4. Integration with existing build system
5. Tests for all minting operations

## Acceptance Criteria

- [ ] Can mint a plugin package from a plugin directory
- [ ] Version bumping works correctly
- [ ] Package validation catches common issues
- [ ] Generated packages can be installed and used
- [ ] Documentation covers all use cases
- [ ] Tests verify package integrity

## Notes

Coordinate with provisioner and harness tools to ensure minted packages work with the full plugin lifecycle.
