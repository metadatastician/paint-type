#!/usr/bin/env python3
"""
Fix literal \n artifacts in manifest and README files.
"""
import os
import re
import sys
from pathlib import Path

def fix_file(filepath):
    """Fix literal \\n artifacts in a file."""
    try:
        content = filepath.read_text(encoding='utf-8')
        
        # Check if file has literal \n
        if '\\n' in content:
            # Replace literal \n with actual newlines
            new_content = content.replace('\\n', '\n')
            
            if new_content != content:
                filepath.write_text(new_content, encoding='utf-8')
                print(f"Fixed: {filepath}")
                return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    repo_root = Path(__file__).parent.parent
    
    # Find all .a2ml and README.adoc files
    files_to_check = []
    for pattern in ['*.a2ml', 'README.adoc']:
        files_to_check.extend(repo_root.rglob(pattern))
    
    # Filter out .git directory
    files_to_check = [f for f in files_to_check if '.git' not in str(f)]
    
    fixed_count = 0
    total_count = len(files_to_check)
    
    print(f"Checking {total_count} files for \\n artifacts...")
    
    for filepath in files_to_check:
        if fix_file(filepath):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files out of {total_count} checked.")
    
    # Verify no more \n artifacts
    remaining = []
    for filepath in files_to_check:
        try:
            content = filepath.read_text(encoding='utf-8')
            if '\\n' in content:
                remaining.append(str(filepath))
        except:
            pass
    
    if remaining:
        print(f"\nWARNING: {len(remaining)} files still have \\n artifacts:")
        for f in remaining[:10]:  # Show first 10
            print(f"  {f}")
        if len(remaining) > 10:
            print(f"  ... and {len(remaining) - 10} more")
        sys.exit(1)
    else:
        print("\nAll files verified - no more \\n artifacts found!")
        sys.exit(0)

if __name__ == '__main__':
    main()
