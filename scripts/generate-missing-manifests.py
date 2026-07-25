#!/usr/bin/env python3
"""
Generate missing AI-MANIFEST.a2ml and README.adoc files for project directories.
"""
import os
import re
import sys
from pathlib import Path

# Directories to prune (same as verify-manifests.yml)
PRUNE_DIRS = {
    '.git', 'node_modules', 'target', 'dist', 'build', '.zig-cache',
    '.venv', '.cache', '.local', '.mypy_cache', '.pytest_cache',
    '.tox', '.nox', '.idea', '.vscode', '__pycache__', '.egg-info',
    '.machine_readable', 'third_party'
}

# Also prune these at any level
ALWAYS_PRUNE = {'.git', 'node_modules', 'target', 'dist', 'build'}

def is_pruned_dir(dirname):
    """Check if a directory should be pruned."""
    return dirname in PRUNE_DIRS or dirname.startswith('.') or dirname.endswith('~')

def determine_depth(repo_root, dir_path):
    """Determine the depth of a directory relative to repo root."""
    rel_path = str(dir_path.relative_to(repo_root))
    if rel_path == '.':
        return 0
    parts = rel_path.split(os.sep)
    return len(parts)

def get_manifest_name(depth):
    """Get the manifest filename for a given depth."""
    if depth == 0:
        return '0-AI-MANIFEST.a2ml'
    return f'0.{depth}-AI-MANIFEST.a2ml'

def get_file_description(filename):
    """Get a description for a file based on its name."""
    if filename == 'Cargo.toml':
        return 'Rust package manifest'
    elif filename == 'Cargo.lock':
        return 'Rust dependency lock file'
    elif filename == 'build.rs':
        return 'Rust build script'
    elif filename == '.gitignore':
        return 'Git ignore rules'
    elif filename == 'README.adoc':
        return 'Directory documentation'
    elif filename == 'LICENSE':
        return 'License file'
    elif filename.endswith('.rs'):
        return 'Rust source code'
    elif filename.endswith('.zig'):
        return 'Zig source code'
    elif filename.endswith('.py'):
        return 'Python script'
    elif filename.endswith('.sh'):
        return 'Shell script'
    elif filename.endswith('.toml'):
        return 'TOML configuration'
    elif filename.endswith('.json'):
        return 'JSON configuration'
    elif filename.endswith(('.yml', '.yaml')):
        return 'YAML configuration'
    elif filename == 'Makefile':
        return 'Make build file'
    elif filename == 'Justfile':
        return 'Just build file'
    elif filename.endswith('.md'):
        return 'Markdown documentation'
    else:
        return 'Project file'

def get_directory_description(dirname):
    """Get a description for a directory based on its name."""
    descriptions = {
        'paint_core': 'Core painting functionality and data structures',
        'paint_collab': 'CvRDT collaboration crate (tile CRDT, permission model, session, Groove transport, LLM gating)',
        'ptype_format': '.ptype image container format — encode/decode Rust crate with fuzzing',
        'aspects': 'Cross-cutting concerns (security, integrity, observability)',
        'host': 'Host-side integration and platform abstraction',
        'host_core': 'Host-side platform abstraction and integration',
        'interface': 'External interfaces (FFI, ABI, gRPC, REST, etc.)',
        'backends': 'Backend implementations (CPU, GPU, crypto, etc.)',
        'shell': 'Desktop shell and user interface',
        'contracts': 'Smart contracts and formal specifications',
        'ephapax': 'Ephapax consensus protocol implementation',
        'affinescript': 'AffineScript language implementation',
        'plugins': 'Plugin system for extendable functionality',
        'definitions': 'Type definitions and domain models',
        'capability': 'Capability-based security system',
        'errors': 'Error handling and reporting',
        'ui': 'User interface components',
        'wasm': 'WebAssembly integration',
        'tools': 'Development and build tools',
        'abi-codegen': 'ABI code generation tool',
        'src': 'Source code directory',
        'tests': 'Test files',
        'examples': 'Example usage',
        'scripts': 'Utility scripts',
        'fuzz': 'Fuzzing targets and seeds',
        'benches': 'Benchmark tests',
        'target': 'Build artifacts',
        'zig-out': 'Zig build output',
        'lib': 'Library files',
    }
    return descriptions.get(dirname, f'This manifest describes the purpose and structure of the directory.')

def get_purpose_from_parent(repo_root, dir_path):
    """Try to extract purpose from parent directory's manifest."""
    parent = dir_path.parent
    if parent == repo_root:
        return None
    
    depth = determine_depth(repo_root, parent)
    manifest_name = get_manifest_name(depth)
    parent_manifest = parent / manifest_name
    
    if parent_manifest.exists():
        try:
            content = parent_manifest.read_text(encoding='utf-8')
            dirname = dir_path.name + '/'
            # Look for the directory in the subdirectories table
            lines = content.split('\n')
            for i, line in enumerate(lines):
                if dirname in line and '|' in line:
                    # Extract the purpose from the table row
                    parts = [p.strip() for p in line.split('|')]
                    if len(parts) >= 3:
                        return parts[2] or ''
        except:
            pass
    return None

def generate_manifest(repo_root, dir_path):
    """Generate a manifest file for a directory."""
    depth = determine_depth(repo_root, dir_path)
    manifest_name = get_manifest_name(depth)
    manifest_path = dir_path / manifest_name
    
    dirname = dir_path.name
    display_name = str(dir_path.relative_to(repo_root))
    
    # Get subdirectories
    subdirs = []
    subdir_purposes = []
    
    for item in dir_path.iterdir():
        if item.is_dir() and not is_pruned_dir(item.name):
            subdirs.append(item.name + '/')
            # Try to get purpose from existing manifest or README
            purpose = get_purpose_from_parent(repo_root, item)
            if not purpose:
                sub_manifest = item / get_manifest_name(depth + 1)
                if sub_manifest.exists():
                    try:
                        content = sub_manifest.read_text(encoding='utf-8')
                        lines = content.split('\n')
                        for line in lines:
                            if line.startswith('## Purpose') and len(lines) > lines.index(line) + 1:
                                purpose = lines[lines.index(line) + 1].strip()
                                break
                    except:
                        pass
            subdir_purposes.append(purpose or '')
    
    # Get canonical files
    canonical_files = []
    common_files = ['Cargo.toml', 'Cargo.lock', 'build.rs', '.gitignore', 'README.adoc', 'LICENSE']
    
    for fname in common_files:
        fpath = dir_path / fname
        if fpath.exists() and fpath.is_file():
            canonical_files.append(fname)
    
    # If no common files, get any non-hidden files
    if not canonical_files:
        for item in dir_path.iterdir():
            if item.is_file() and not item.name.startswith('.') and not item.name.endswith('~'):
                canonical_files.append(item.name)
    
    # Get purpose
    purpose = get_directory_description(dirname)
    if not purpose or purpose == 'This manifest describes the purpose and structure of the directory.':
        parent_purpose = get_purpose_from_parent(repo_root, dir_path)
        if parent_purpose:
            purpose = parent_purpose
    
    # Build manifest content
    lines = []
    lines.append('# SPDX-License-Identifier: AGPL-3.0-or-later')
    lines.append('')
    lines.append(f'# paint.type AI Manifest (Level {depth})')
    lines.append('')
    lines.append(f'This is the AI manifest for the `{display_name}/` directory (Level {depth}) of paint.type.')
    lines.append('')
    lines.append('## Purpose')
    lines.append('')
    lines.append(purpose)
    lines.append('')
    lines.append('## Authority Split')
    lines.append('')
    lines.append('- **Central protocol authority:** `standards/` repository')
    lines.append(f'- **Local integration authority (this directory):** {display_name}/ specific configuration')
    lines.append('')
    
    if subdirs:
        lines.append('## Subdirectories')
        lines.append('')
        lines.append('| Directory | Layer | Purpose |')
        lines.append('|---|---|---|')
        for subdir, sub_purpose in zip(subdirs, subdir_purposes):
            lines.append(f'| `{subdir}` | {depth + 1} | {sub_purpose} |')
        lines.append('')
    
    if canonical_files:
        lines.append('## Canonical Files')
        lines.append('')
        for fname in canonical_files:
            desc = get_file_description(fname)
            lines.append(f'- `{fname}` — {desc}')
        lines.append('')
    
    lines.append('## Invariants')
    lines.append('')
    lines.append('1. All A2ML state files must live in `.machine_readable/` ONLY, never in the repo root.')
    if depth == 0:
        lines.append('2. The root directory must have a `0-AI-MANIFEST.a2ml`.')
    else:
        lines.append(f'2. Each directory at layer {depth} must have its own `0.{depth}-AI-MANIFEST.a2ml`.')
    lines.append('3. Each directory must have a `README.adoc` describing purpose, contents, and function.')
    lines.append('4. SPDX license identifiers must be consistent across all files (AGPL-3.0-or-later).')
    lines.append('')
    lines.append('## Startup Checklist for Agents')
    lines.append('')
    lines.append(f'1. Read this file (`{manifest_name}`).')
    lines.append('2. Read `README.adoc` for directory overview.')
    lines.append('3. For subdirectories, navigate to their respective manifest files.')
    lines.append('4. Never edit `.machine_readable/` files directly.')
    
    manifest_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    return manifest_path

def generate_readme(repo_root, dir_path):
    """Generate a README.adoc file for a directory."""
    depth = determine_depth(repo_root, dir_path)
    readme_path = dir_path / 'README.adoc'
    
    dirname = dir_path.name
    display_name = str(dir_path.relative_to(repo_root))
    
    # Get subdirectories
    subdirs = []
    for item in dir_path.iterdir():
        if item.is_dir() and not is_pruned_dir(item.name):
            subdirs.append(item.name + '/')
    
    # Get description
    description = get_directory_description(dirname)
    
    # Build README content
    lines = []
    lines.append('// SPDX-License-Identifier: CC-BY-SA-4.0')
    lines.append('')
    lines.append(f'= {dirname}')
    lines.append(f':description: {description}')
    lines.append('')
    lines.append('This directory is part of the paint.type project.')
    lines.append('')
    lines.append('== Purpose')
    lines.append('')
    lines.append(description)
    lines.append('')
    lines.append(f'As a Level {depth} directory (`{display_name}/`), it supports the overall')
    lines.append('project structure.')
    lines.append('')
    
    if subdirs:
        lines.append('== Subdirectories')
        lines.append('')
        lines.append('[cols="1,2", options="header"]')
        lines.append('|===')
        lines.append('| Directory | Purpose')
        for subdir in subdirs:
            lines.append(f'| `{subdir}` | |')
        lines.append('|===')
        lines.append('')
    
    lines.append('== Purpose of Each Component')
    lines.append('')
    lines.append('')
    lines.append('== AI Manifest Structure')
    lines.append('')
    manifest_name = get_manifest_name(depth)
    lines.append(f'* `{manifest_name}` (this file) - Level {depth} manifest for `{display_name}/`')
    lines.append('')
    lines.append(f'* Each subdirectory has its own `0.{depth + 1}-AI-MANIFEST.a2ml`')
    lines.append('')
    lines.append('== Related Files')
    lines.append('')
    if depth > 0:
        parent_depth = depth - 1
        parent_manifest = get_manifest_name(parent_depth)
        lines.append(f'* link:../{parent_manifest}[Parent AI Manifest]')
        lines.append('* link:../README.adoc[Parent Directory README]')
    lines.append('* link:../../.machine_readable/[Machine Readable Metadata]')
    
    readme_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    return readme_path

def should_prune_path(repo_root, path):
    """Check if a path should be pruned."""
    parts = str(path.relative_to(repo_root)).split(os.sep)
    for part in parts:
        if part in PRUNE_DIRS or part in ALWAYS_PRUNE:
            return True
    return False

def main():
    repo_root = Path(__file__).parent.parent
    
    # Find all directories
    all_dirs = []
    for dirpath, dirnames, filenames in os.walk(repo_root):
        # Modify dirnames in place to prune subdirectories
        dirnames[:] = [d for d in dirnames if not is_pruned_dir(d)]
        all_dirs.append(Path(dirpath))
    
    # Filter out pruned paths
    dirs_to_check = []
    for dir_path in all_dirs:
        if not should_prune_path(repo_root, dir_path):
            dirs_to_check.append(dir_path)
    
    # Check which directories need manifests
    missing_manifests = []
    missing_readmes = []
    
    for dir_path in dirs_to_check:
        depth = determine_depth(repo_root, dir_path)
        manifest_name = get_manifest_name(depth)
        manifest_path = dir_path / manifest_name
        readme_path = dir_path / 'README.adoc'
        
        manifest_exists = manifest_path.exists()
        readme_exists = readme_path.exists()
        
        # For root, README.md is also acceptable
        if depth == 0 and not readme_exists:
            readme_exists = (dir_path / 'README.md').exists()
        
        if not manifest_exists:
            missing_manifests.append(dir_path)
        if not readme_exists:
            missing_readmes.append(dir_path)
    
    print(f"Found {len(missing_manifests)} directories missing manifests")
    print(f"Found {len(missing_readmes)} directories missing READMEs")
    print()
    
    # Generate missing files
    created_manifests = 0
    created_readmes = 0
    
    for dir_path in dirs_to_check:
        depth = determine_depth(repo_root, dir_path)
        manifest_name = get_manifest_name(depth)
        manifest_path = dir_path / manifest_name
        readme_path = dir_path / 'README.adoc'
        
        manifest_exists = manifest_path.exists()
        readme_exists = readme_path.exists()
        
        # For root, README.md is also acceptable
        if depth == 0 and not readme_exists:
            readme_exists = (dir_path / 'README.md').exists()
        
        if not manifest_exists:
            print(f"Creating manifest: {manifest_path}")
            generate_manifest(repo_root, dir_path)
            created_manifests += 1
        
        if not readme_exists:
            print(f"Creating README: {readme_path}")
            generate_readme(repo_root, dir_path)
            created_readmes += 1
    
    print()
    print(f"Created {created_manifests} manifests and {created_readmes} READMEs")

if __name__ == '__main__':
    main()
