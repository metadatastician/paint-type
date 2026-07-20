# Plugin Wizard for paint-type

**Status:** Open  
**Priority:** High  
**Labels:** enhancement, plugin-system, tooling  
**Depends on:** #00 (Overview Issue)  

## Description

Create an interactive CLI wizard tool for scaffolding new paint-type plugins. This tool should guide users through the process of creating a new plugin with appropriate structure, metadata, and boilerplate code.

## Inspiration

Examine similar tools in:
- `boj-server/boj-server` cartridges
- `panll` plugin system
- Other estate repositories with wizard tooling

## Requirements

### Functionality
- [ ] Interactive prompt-based interface
- [ ] Support for different plugin types (effect, filter, transform, etc.)
- [ ] Generate plugin manifest with metadata (name, version, description, author)
- [ ] Create directory structure following paint-type conventions
- [ ] Generate boilerplate code based on plugin type
- [ ] Support for multiple languages (Rust, Zig, Idris2, etc.)
- [ ] Validate inputs before generation
- [ ] Dry-run mode to preview what will be created

### Integration
- [ ] Integrate with paint-type's build system (Justfile, Makefile, etc.)
- [ ] Add as a subcommand to paint-type CLI if applicable
- [ ] Provide standalone executable option

### User Experience
- [ ] Clear prompts with sensible defaults
- [ ] Help text for each option
- [ ] Ability to abort and start over
- [ ] Progress feedback during generation

## Deliverables

1. `tools/plugin-wizard/` directory with implementation
2. CLI command: `paint-type plugin new` or `plugin-wizard`
3. Documentation in `docs/tools/plugin-wizard.adoc`
4. Tests for wizard functionality
5. Example generated plugins for verification

## Acceptance Criteria

- [ ] Wizard can create a minimal functional plugin
- [ ] All plugin types supported by paint-type are covered
- [ ] Generated plugins pass basic validation
- [ ] Documentation is complete
- [ ] Tests cover all major code paths

## Notes

This is part of the broader plugin toolset initiative. Coordinate with other tooling issues (minter, provisioner, configurator, harness).
