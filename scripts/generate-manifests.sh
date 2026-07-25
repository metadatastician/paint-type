#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Manifest Generator Script
# Generates AI-MANIFEST.a2ml and README.adoc files for all project directories
#
# Usage: ./generate-manifests.sh [OPTIONS]
#
# Options:
#   --dry-run     Only show what would be created, don't actually create files
#   --force       Overwrite existing files (default: skip existing)
#   --all         Generate for ALL directories, including those with existing manifests
#   --missing     Only generate for directories that are missing manifests (default)
#   --fix         Fix existing manifests with \n artifacts
#   --help        Show this help message
#
# The script uses templates from docs/templates/ and populates them with
# directory-specific information.

set -euo pipefail

# Default options
DRY_RUN=false
FORCE=false
GENERATE_ALL=false
FIX_MODE=false
VERBOSE=false

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --dry-run)
            DRY_RUN=true
            ;;
        --force)
            FORCE=true
            ;;
        --all)
            GENERATE_ALL=true
            ;;
        --missing)
            GENERATE_ALL=false
            ;;
        --fix)
            FIX_MODE=true
            ;;
        --verbose)
            VERBOSE=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dry-run     Only show what would be created"
            echo "  --force       Overwrite existing files"
            echo "  --all         Generate for ALL directories"
            echo "  --missing     Only generate for missing manifests (default)"
            echo "  --fix         Fix existing manifests with \\n artifacts"
            echo "  --verbose     Show detailed output"
            echo "  --help        Show this help message"
            exit 0
            ;;
        *)
            echo "Error: Unknown argument '$arg'"
            exit 1
            ;;
    esac
done

# Get repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Counters
CREATED_MANIFESTS=0
CREATED_READMES=0
FIXED_MANIFESTS=0
SKIPPED=0
TOTAL=0

# Function to log messages
log() {
    if [ "$VERBOSE" = true ]; then
        echo "[VERBOSE] $1"
    fi
}

# Function to fix \n artifacts in a file
fix_newline_artifacts() {
    local file="$1"
    
    # Check if file has literal \n artifacts (backslash followed by n)
    if grep -q '\\n' "$file" 2>/dev/null; then
        log "Fixing \\n artifacts in $file"
        
        # Use sed to replace literal \n with actual newlines
        # We use a two-step process to avoid sed interpretation issues
        local tmp_file="${file}.tmp"
        
        # First, replace \n with a placeholder that won't appear in the file
        sed 's/\\n/___NEWLINE___/g' "$file" > "$tmp_file"
        
        # Then replace the placeholder with actual newlines
        # Note: echo interprets \n, so we use printf to output a literal newline
        sed 's/___NEWLINE___/\n/g' "$tmp_file" > "$file"
        rm -f "$tmp_file"
        
        echo "  Fixed: $file"
        FIXED_MANIFESTS=$((FIXED_MANIFESTS + 1))
    fi
}

# Function to determine the depth of a directory
determine_depth() {
    local dir="$1"
    local rel_path="${dir#$REPO_ROOT/}"
    
    if [ -z "$rel_path" ] || [ "$rel_path" = "." ]; then
        echo 0
    else
        # Count the number of path components
        echo "$rel_path" | tr '/' '\n' | grep -v '^$' | wc -l
    fi
}

# Function to get directory name for display
dir_display_name() {
    local dir="$1"
    local rel_path="${dir#$REPO_ROOT/}"
    if [ -z "$rel_path" ]; then
        echo "/"
    else
        echo "$rel_path"
    fi
}

# Function to generate a manifest for a specific directory
generate_directory_manifests() {
    local dir="$1"
    local depth=$(determine_depth "$dir")
    local display_name=$(dir_display_name "$dir")
    
    TOTAL=$((TOTAL + 1))
    
    # Determine manifest filename based on depth
    local manifest_name
    if [ "$depth" -eq 0 ]; then
        manifest_name="0-AI-MANIFEST.a2ml"
    else
        manifest_name="0.${depth}-AI-MANIFEST.a2ml"
    fi
    
    local manifest_path="$dir/$manifest_name"
    local readme_path="$dir/README.adoc"
    
    # Check if manifest already exists
    local manifest_exists=false
    local readme_exists=false
    
    if [ -f "$manifest_path" ]; then
        manifest_exists=true
    fi
    
    if [ -f "$readme_path" ]; then
        readme_exists=true
    fi
    
    # Check if we should generate for this directory
    local should_generate=false
    
    if [ "$GENERATE_ALL" = true ]; then
        should_generate=true
    elif [ "$manifest_exists" = false ] || [ "$readme_exists" = false ]; then
        should_generate=true
    fi
    
    # If in fix mode, check for \n artifacts in existing manifests
    # This should be done regardless of whether we're generating new files
    if [ "$FIX_MODE" = true ]; then
        if [ "$manifest_exists" = true ]; then
            fix_newline_artifacts "$manifest_path"
        fi
        if [ "$readme_exists" = true ]; then
            fix_newline_artifacts "$readme_path"
        fi
    fi
    
    # If we should generate and (force or not exists)
    if [ "$should_generate" = true ]; then
        local create_manifest=false
        local create_readme=false
        
        if [ "$FORCE" = true ] || [ "$manifest_exists" = false ]; then
            create_manifest=true
        fi
        
        if [ "$FORCE" = true ] || [ "$readme_exists" = false ]; then
            create_readme=true
        fi
        
        # Only create if not dry run
        if [ "$DRY_RUN" = false ]; then
            # Create manifest
            if [ "$create_manifest" = true ]; then
                generate_manifest_file "$dir" "$depth" "$manifest_path"
                CREATED_MANIFESTS=$((CREATED_MANIFESTS + 1))
            else
                log "Skipping existing manifest: $manifest_path"
            fi
            
            # Create README
            if [ "$create_readme" = true ]; then
                generate_readme_file "$dir" "$depth" "$readme_path"
                CREATED_READMES=$((CREATED_READMES + 1))
            else
                log "Skipping existing README: $readme_path"
            fi
        else
            # Dry run - just report
            if [ "$create_manifest" = true ]; then
                echo "[DRY RUN] Would create: $manifest_path"
            fi
            if [ "$create_readme" = true ]; then
                echo "[DRY RUN] Would create: $readme_path"
            fi
        fi
    else
        SKIPPED=$((SKIPPED + 1))
        log "Skipping compliant directory: $display_name"
    fi
}

# Function to generate manifest file content
generate_manifest_file() {
    local dir="$1"
    local depth="$2"
    local manifest_path="$3"
    
    local display_name=$(dir_display_name "$dir")
    local basename=$(basename "$dir")
    
    # Get subdirectories
    local subdirs=()
    local subdir_purposes=()
    
    if [ -d "$dir" ]; then
        while IFS= read -r -d '' subdir; do
            # Skip hidden directories
            local subdir_name=$(basename "$subdir")
            if [[ "$subdir_name" == .* ]]; then
                continue
            fi
            
            # Skip common non-content directories
            case "$subdir_name" in
                target|node_modules|.git|.DS_Store|build|dist|__pycache__|*.egg-info|.venv)
                    continue
                    ;;
            esac
            
            subdirs+=("$subdir_name")
            # Try to get purpose from existing README or manifest
            local purpose=""
            local sub_manifest="$subdir/0.$((depth + 1))-AI-MANIFEST.a2ml"
            if [ -f "$sub_manifest" ]; then
                purpose=$(grep -A 2 "## Purpose" "$sub_manifest" | tail -1 | sed 's/^[[:space:]]*//')
            fi
            if [ -z "$purpose" ]; then
                local sub_readme="$subdir/README.adoc"
                if [ -f "$sub_readme" ]; then
                    purpose=$(head -5 "$sub_readme" | grep "Purpose\|Description" | sed 's/^[[:space:]]*//' | head -1)
                fi
            fi
            subdir_purposes+=("$purpose")
        done < <(find "$dir" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)
    fi
    
    # Get canonical files
    local canonical_files=()
    local common_files=(Cargo.toml Cargo.lock build.rs .gitignore README.adoc LICENSE)
    
    for f in "${common_files[@]}"; do
        if [ -f "$dir/$f" ]; then
            canonical_files+=("$f")
        fi
    done
    
    # If no specific files found, look for any non-hidden files
    if [ ${#canonical_files[@]} -eq 0 ]; then
        while IFS= read -r -d '' f; do
            local fname=$(basename "$f")
            if [[ "$fname" != .* ]] && [[ "$fname" != *~ ]] && [[ "$fname" != *.swp ]]; then
                canonical_files+=("$fname")
            fi
        done < <(find "$dir" -maxdepth 1 -type f -print0 2>/dev/null)
    fi
    
    # Determine purpose based on directory name and contents
    local purpose=""
    case "$basename" in
        paint_core)
            purpose="Core painting functionality and data structures"
            ;;
        paint_collab)
            purpose="CvRDT collaboration crate (tile CRDT, permission model, session, Groove transport, LLM gating)"
            ;;
        ptype_format)
            purpose=".ptype image container format — encode/decode Rust crate with fuzzing"
            ;;
        aspects)
            purpose="Cross-cutting concerns (security, integrity, observability)"
            ;;
        host|host_core)
            purpose="Host-side integration and platform abstraction"
            ;;
        interface)
            purpose="External interfaces (FFI, ABI, gRPC, REST, etc.)"
            ;;
        backends)
            purpose="Backend implementations (CPU, GPU, crypto, etc.)"
            ;;
        shell)
            purpose="Desktop shell and user interface"
            ;;
        contracts)
            purpose="Smart contracts and formal specifications"
            ;;
        ephapax)
            purpose="Ephapax consensus protocol implementation"
            ;;
        affinescript)
            purpose="AffineScript language implementation"
            ;;
        plugins)
            purpose="Plugin system for extendable functionality"
            ;;
        definitions)
            purpose="Type definitions and domain models"
            ;;
        capability)
            purpose="Capability-based security system"
            ;;
        errors)
            purpose="Error handling and reporting"
            ;;
        ui)
            purpose="User interface components"
            ;;
        wasm)
            purpose="WebAssembly integration"
            ;;
        tools)
            purpose="Development and build tools"
            ;;
        *)
            # Try to read purpose from parent directory
            local parent_depth=$((depth - 1))
            local parent_dir=$(dirname "$dir")
            if [ "$parent_depth" -ge 0 ]; then
                local parent_manifest="$parent_dir/0.${parent_depth}-AI-MANIFEST.a2ml"
                if [ -f "$parent_manifest" ]; then
                    # Extract from subdirectories table if exists
                    purpose=$(grep -A 10 "## Subdirectories" "$parent_manifest" | grep "| ${basename}/ |" | sed 's/^[[:space:]]*|[[:space:]]*${basename}\/[[:space:]]*|[[:space:]]*[0-9][[:space:]]*|[[:space:]]*//' | sed 's/[[:space:]]*|[[:space:]]*$//')
                fi
            fi
            if [ -z "$purpose" ]; then
                purpose="This manifest describes the purpose and structure of the \`${display_name}/\` directory."
            fi
            ;;
    esac
    
    # Escape special characters for echo
    local safe_display_name=$(echo "$display_name" | sed 's/\//\\\//g')
    local safe_purpose=$(echo "$purpose" | sed 's/\//\\\//g')
    
    # Create the manifest file
    {
        echo "# SPDX-License-Identifier: AGPL-3.0-or-later"
        echo ""
        echo "# paint.type AI Manifest (Level ${depth})"
        echo ""
        echo "This is the AI manifest for the \`${display_name}/\` directory (Level ${depth}) of paint.type."
        echo ""
        echo "## Purpose"
        echo ""
        echo "${purpose}"
        echo ""
        echo "## Authority Split"
        echo ""
        echo "- **Central protocol authority:** \`standards/\` repository"
        echo "- **Local integration authority (this directory):** ${display_name}/ specific configuration"
        echo ""
        
        if [ ${#subdirs[@]} -gt 0 ]; then
            echo "## Subdirectories"
            echo ""
            echo "| Directory | Layer | Purpose |"
            echo "|---|---|---|"
            for i in "${!subdirs[@]}"; do
                local subdir="${subdirs[$i]}"
                local sub_purpose="${subdir_purposes[$i]}"
                if [ -z "$sub_purpose" ]; then
                    sub_purpose=" "
                fi
                echo "| \`${subdir}/\` | $((depth + 1)) | ${sub_purpose} |"
            done
            echo ""
        fi
        
        if [ ${#canonical_files[@]} -gt 0 ]; then
            echo "## Canonical Files"
            echo ""
            for f in "${canonical_files[@]}"; do
                echo "- \`${f}\` — $(file_description "$f")"
            done
            echo ""
        fi
        
        echo "## Invariants"
        echo ""
        echo "1. All A2ML state files must live in \`.machine_readable/\` ONLY, never in the repo root."
        if [ "$depth" -eq 0 ]; then
            echo "2. The root directory must have a \`0-AI-MANIFEST.a2ml\`."
        else
            echo "2. Each directory at layer ${depth} must have its own \`0.${depth}-AI-MANIFEST.a2ml\`."
        fi
        echo "3. Each directory must have a \`README.adoc\` describing purpose, contents, and function."
        echo "4. SPDX license identifiers must be consistent across all files (AGPL-3.0-or-later)."
        echo ""
        echo "## Startup Checklist for Agents"
        echo ""
        echo "1. Read this file (\`${manifest_name}\`)."
        echo "2. Read \`README.adoc\` for directory overview."
        echo "3. For subdirectories, navigate to their respective manifest files."
        echo "4. Never edit \`.machine_readable/\` files directly."
    } > "$manifest_path"
    
    echo "  Created: $manifest_path"
}

# Function to get file description
file_description() {
    local filename="$1"
    case "$filename" in
        Cargo.toml) echo "Rust package manifest" ;;
        Cargo.lock) echo "Rust dependency lock file" ;;
        build.rs) echo "Rust build script" ;;
        .gitignore) echo "Git ignore rules" ;;
        README.adoc) echo "Directory documentation" ;;
        LICENSE) echo "License file" ;;
        *.rs) echo "Rust source code" ;;
        *.zig) echo "Zig source code" ;;
        *.py) echo "Python script" ;;
        *.sh) echo "Shell script" ;;
        *.toml) echo "TOML configuration" ;;
        *.json) echo "JSON configuration" ;;
        *.yml|*.yaml) echo "YAML configuration" ;;
        Makefile) echo "Make build file" ;;
        Justfile) echo "Just build file" ;;
        *.md) echo "Markdown documentation" ;;
        *) echo "Project file" ;;
    esac
}

# Function to generate README file content
generate_readme_file() {
    local dir="$1"
    local depth="$2"
    local readme_path="$3"
    
    local display_name=$(dir_display_name "$dir")
    local basename=$(basename "$dir")
    
    # Get subdirectories
    local subdirs=()
    
    if [ -d "$dir" ]; then
        while IFS= read -r -d '' subdir; do
            local subdir_name=$(basename "$subdir")
            if [[ "$subdir_name" == .* ]]; then
                continue
            fi
            case "$subdir_name" in
                target|node_modules|.git|.DS_Store|build|dist|__pycache__|*.egg-info|.venv)
                    continue
                    ;;
            esac
            subdirs+=("$subdir_name")
        done < <(find "$dir" -mindepth 1 -maxdepth 1 -type d -print0 2>/dev/null)
    fi
    
    # Determine description based on directory
    local description=""
    case "$basename" in
        paint_core)
            description="Core painting functionality crate"
            ;;
        paint_collab)
            description="CvRDT collaboration crate for real-time multi-user editing"
            ;;
        ptype_format)
            description=".ptype image container format crate"
            ;;
        aspects)
            description="Cross-cutting concerns and aspect-oriented components"
            ;;
        host|host_core)
            description="Host-side platform abstraction and integration"
            ;;
        interface)
            description="External API and interface definitions"
            ;;
        backends)
            description="Backend implementations for various compute targets"
            ;;
        shell)
            description="Desktop shell application"
            ;;
        contracts)
            description="Formal specifications and smart contracts"
            ;;
        ephapax)
            description="Ephapax consensus protocol implementation"
            ;;
        affinescript)
            description="AffineScript programming language"
            ;;
        plugins)
            description="Plugin system for extensible functionality"
            ;;
        definitions)
            description="Type definitions and domain models"
            ;;
        capability)
            description="Capability-based security framework"
            ;;
        errors)
            description="Error types and handling utilities"
            ;;
        ui)
            description="User interface components and widgets"
            ;;
        wasm)
            description="WebAssembly integration and bindings"
            ;;
        tools)
            description="Development and build utilities"
            ;;
        *)
            description="This directory is part of the paint.type project."
            ;;
    esac
    
    local parent_dir=$(dirname "$dir")
    
    # Create the README file
    {
        echo "// SPDX-License-Identifier: CC-BY-SA-4.0"
        echo ""
        echo "= ${basename}"
        echo ":description: ${description}"
        echo ""
        echo "This directory is part of the paint.type project."
        echo ""
        echo "== Purpose"
        echo ""
        echo "${description}"
        echo ""
        echo "As a Level ${depth} directory (\`${display_name}/\`), it supports the overall"
        echo "project structure."
        echo ""
        
        if [ ${#subdirs[@]} -gt 0 ]; then
            echo "== Subdirectories"
            echo ""
            echo "[cols=\"1,2\", options=\"header\"]"
            echo "|==="
            echo "| Directory | Purpose"
            for subdir in "${subdirs[@]}"; do
                echo "| \`${subdir}/\` | |"
            done
            echo "|==="
            echo ""
        fi
        
        echo "== Purpose of Each Component"
        echo ""
        echo ""
        echo "== AI Manifest Structure"
        echo ""
        echo "* \`0.${depth}-AI-MANIFEST.a2ml\` (this file) - Level ${depth} manifest for \`${display_name}/\`"
        echo ""
        echo "* Each subdirectory has its own \`0.$((depth + 1))-AI-MANIFEST.a2ml\`"
        echo ""
        echo "== Related Files"
        echo ""
        if [ "$depth" -gt 0 ]; then
            local parent_depth=$((depth - 1))
            local parent_manifest_name
            if [ "$parent_depth" -eq 0 ]; then
                parent_manifest_name="0-AI-MANIFEST.a2ml"
            else
                parent_manifest_name="0.${parent_depth}-AI-MANIFEST.a2ml"
            fi
            echo "* link:../${parent_manifest_name}[Parent AI Manifest]"
            echo "* link:../README.adoc[Parent Directory README]"
        fi
        echo "* link:../../.machine_readable/[Machine Readable Metadata]"
    } > "$readme_path"
    
    echo "  Created: $readme_path"
}

# Main function to walk the directory tree
walk_and_generate() {
    echo "=== Manifest Generation ==="
    echo "Repository: $REPO_ROOT"
    echo "Options: dry-run=$DRY_RUN, force=$FORCE, all=$GENERATE_ALL, fix=$FIX_MODE"
    echo ""
    
    # Prune list - directories to skip
    local prune_dirs=(".git" ".machine_readable" "node_modules" "target" "dist" "build" ".zig-cache" ".venv" ".cache" ".local" ".mypy_cache" ".pytest_cache" ".tox" ".nox" ".idea" ".vscode" "__pycache__" ".egg-info" "third_party")
    
    # Build find command to skip pruned directories
    local find_cmd="find \"$REPO_ROOT\""
    for prune_dir in "${prune_dirs[@]}"; do
        find_cmd+=" -path \"*/${prune_dir}/*\" -prune -o"
    done
    find_cmd+=" -type d -print0"
    
    # Process each directory
    while IFS= read -r -d '' dir; do
        # Skip the repo root if we're not generating all (it has special handling)
        if [ "$dir" = "$REPO_ROOT" ] && [ "$GENERATE_ALL" = false ]; then
            continue
        fi
        
        generate_directory_manifests "$dir"
    done < <(eval "$find_cmd")
    
    # Summary
    echo ""
    echo "=== Generation Summary ==="
    echo "Total directories processed: $TOTAL"
    echo "Manifests created: $CREATED_MANIFESTS"
    echo "READMEs created: $CREATED_READMES"
    echo "Manifests fixed: $FIXED_MANIFESTS"
    echo "Skipped: $SKIPPED"
}

# Run main function
walk_and_generate

exit 0
