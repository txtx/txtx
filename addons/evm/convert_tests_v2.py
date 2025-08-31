#!/usr/bin/env python3
"""Convert integration tests from ProjectTestHarness to FixtureBuilder - v2"""

import re
import os
from pathlib import Path

def convert_test_file(filepath):
    """Convert a single test file"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Skip mod.rs and anvil_harness.rs
    if filepath.name in ['mod.rs', 'anvil_harness.rs']:
        print(f"⊘ Skipping: {filepath.name}")
        return False
    
    # Skip if already properly converted
    if 'MigrationHelper' in content and '.execute()' in content and '.await' in content:
        print(f"✓ Already converted: {filepath.name}")
        return False
    
    original_content = content
    
    # Replace imports
    content = re.sub(
        r'use crate::tests::test_harness::ProjectTestHarness;',
        'use crate::tests::fixture_builder::MigrationHelper;',
        content
    )
    
    # Add tokio import if needed
    if '#[test]' in content and 'use tokio;' not in content:
        # Find the first use statement in the test module
        match = re.search(r'(mod \w+ \{[^}]*?)(use )', content, re.DOTALL)
        if match:
            content = content[:match.end(1)] + 'use tokio;\n    ' + content[match.end(1):]
    
    # Convert test functions to async tokio tests
    content = re.sub(
        r'#\[test\](\s+)fn (\w+)\(\)',
        r'#[tokio::test]\1async fn \2()',
        content
    )
    
    # Pattern 1: Simple harness creation and execution
    pattern1 = re.compile(
        r'let harness = ProjectTestHarness::from_fixture\(&fixture_path\)((?:\s*\.with_input\([^)]+\))*)\s*;?\s*'
        r'let result = harness\.execute_runbook\(\)\s*\.expect\([^)]+\);',
        re.DOTALL
    )
    
    def replace1(match):
        inputs = match.group(1)
        return f'let result = MigrationHelper::from_fixture(&fixture_path){inputs}\n            .execute()\n            .await\n            .expect("Failed to execute runbook");'
    
    content = pattern1.sub(replace1, content)
    
    # Pattern 2: Harness with separate execution
    pattern2 = re.compile(
        r'let harness = ProjectTestHarness::from_fixture\(&fixture_path\)((?:\s*\.with_input\([^)]+\))*)\s*;',
        re.DOTALL
    )
    
    def replace2(match):
        inputs = match.group(1)
        return f'let harness = MigrationHelper::from_fixture(&fixture_path){inputs};'
    
    content = pattern2.sub(replace2, content)
    
    # Convert harness.execute_runbook() calls
    content = re.sub(
        r'harness\.execute_runbook\(\)(\s*\.expect\([^)]+\))?',
        r'harness.execute().await\1',
        content
    )
    
    # Handle results that need renaming
    content = re.sub(
        r'let result = harness\.execute\(\)\.await',
        r'let result = harness.execute().await',
        content
    )
    
    # Clean up any Value import issues
    if 'Value' in content and 'txtx_addon_kit::types::types::Value' not in content:
        content = re.sub(
            r'(use crate::tests::fixture_builder::MigrationHelper;)',
            r'\1\n    use txtx_addon_kit::types::types::Value;',
            content
        )
    
    # Only write if we made changes
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"✅ Converted: {filepath.name}")
        return True
    else:
        print(f"⊘ No changes: {filepath.name}")
        return False

def main():
    test_dir = Path('/home/amal/dev/tx/txtx/addons/evm/src/tests/integration')
    
    converted = 0
    skipped = 0
    
    for filepath in sorted(test_dir.glob('*.rs')):
        if convert_test_file(filepath):
            converted += 1
        else:
            skipped += 1
    
    print(f"\n📊 Summary: {converted} files converted, {skipped} files skipped/unchanged")

if __name__ == '__main__':
    main()