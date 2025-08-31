#!/usr/bin/env python3
"""Fix compilation errors in converted test files"""

import re
from pathlib import Path

def fix_file(filepath):
    """Fix common patterns causing compilation errors"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # Fix pattern: let harness = MigrationHelper -> let result = MigrationHelper
    # But only when followed by assert!(result.
    if 'let harness = MigrationHelper' in content and 'assert!(result.' in content:
        content = re.sub(
            r'let harness = MigrationHelper',
            r'let result = MigrationHelper',
            content
        )
    
    # Fix pattern: duplicate .execute().await calls
    content = re.sub(
        r'\.execute\(\)\s*\.await\s*\.expect\([^)]+\);?\s*let \w+_result = \w+_result\.execute\(\)\.await',
        r'.execute()\n            .await',
        content
    )
    
    # Fix pattern: ProjectTestHarness -> MigrationHelper
    content = re.sub(
        r'ProjectTestHarness::from_fixture',
        r'MigrationHelper::from_fixture',
        content
    )
    
    # Remove mut from MigrationHelper declarations
    content = re.sub(
        r'let mut (\w+) = MigrationHelper',
        r'let \1 = MigrationHelper',
        content
    )
    
    # Fix pattern: harness.execute() -> result variable assignment
    # When we have harness defined but result used
    if 'let harness = ' in content and 'result.success' in content:
        # Find lines like: let harness = ... .expect("...");
        # followed by assert!(result.success
        pattern = re.compile(
            r'let harness = (.*?\.expect\([^)]+\));',
            re.DOTALL
        )
        content = pattern.sub(r'let result = \1;', content)
    
    # Fix deploy_harness -> deploy_result pattern
    content = re.sub(
        r'let mut deploy_harness = MigrationHelper(.*?)\.expect\([^)]+\);',
        r'let deploy_result = MigrationHelper\1.expect("Failed to execute");',
        content,
        flags=re.DOTALL
    )
    
    # Fix call_harness -> call_result pattern
    content = re.sub(
        r'let mut call_harness = MigrationHelper(.*?)\.expect\([^)]+\);',
        r'let call_result = MigrationHelper\1.expect("Failed to execute");',
        content,
        flags=re.DOTALL
    )
    
    # Remove duplicate execution patterns
    content = re.sub(
        r'let (\w+_result) = \1\.execute\(\)\.await',
        r'// Execution already done above',
        content
    )
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"✅ Fixed: {filepath.name}")
        return True
    else:
        print(f"⊘ No changes: {filepath.name}")
        return False

def main():
    test_dir = Path('/home/amal/dev/tx/txtx/addons/evm/src/tests/integration')
    
    fixed = 0
    for filepath in sorted(test_dir.glob('*.rs')):
        if filepath.name in ['mod.rs', 'anvil_harness.rs']:
            continue
        if fix_file(filepath):
            fixed += 1
    
    print(f"\n📊 Fixed {fixed} files")

if __name__ == '__main__':
    main()