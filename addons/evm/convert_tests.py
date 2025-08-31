#!/usr/bin/env python3
"""Convert integration tests from ProjectTestHarness to FixtureBuilder"""

import re
import os
from pathlib import Path

def convert_test_file(filepath):
    """Convert a single test file"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Skip if already converted
    if 'MigrationHelper' in content:
        print(f"✓ Already converted: {filepath}")
        return False
    
    # Skip mod.rs and anvil_harness.rs
    if filepath.name in ['mod.rs', 'anvil_harness.rs']:
        print(f"⊘ Skipping: {filepath}")
        return False
    
    # Replace imports
    content = re.sub(
        r'use crate::tests::test_harness::ProjectTestHarness;',
        'use crate::tests::fixture_builder::{MigrationHelper, TestResult};',
        content
    )
    
    # Add tokio import if not present
    if 'use tokio;' not in content and '#[tokio::test]' not in content:
        content = re.sub(
            r'(use std::path::PathBuf;)',
            r'\1\n    use tokio;',
            content
        )
    
    # Convert test functions to async
    content = re.sub(
        r'#\[test\]\s+fn (\w+)\(\)',
        r'#[tokio::test]\n    async fn \1()',
        content
    )
    
    # Convert ProjectTestHarness::from_fixture pattern
    content = re.sub(
        r'let harness = ProjectTestHarness::from_fixture\(&fixture_path\)',
        r'let result = MigrationHelper::from_fixture(&fixture_path)',
        content
    )
    
    # Convert .with_input chaining
    content = re.sub(
        r'\.with_input\(([^)]+)\);',
        r'.with_input(\1)\n            .execute()\n            .await\n            .expect("Failed to execute test");',
        content
    )
    
    # Convert execute_runbook calls
    content = re.sub(
        r'let result = harness\.execute_runbook\(\)\s*\.expect\([^)]+\);',
        '',
        content
    )
    
    # Remove duplicate result declarations
    content = re.sub(
        r'let result = result\s+',
        '',
        content
    )
    
    # Handle special cases where execute is called separately
    content = re.sub(
        r'harness\.execute_runbook\(\)',
        r'result.execute().await',
        content
    )
    
    # Save converted file
    with open(filepath, 'w') as f:
        f.write(content)
    
    print(f"✅ Converted: {filepath}")
    return True

def main():
    test_dir = Path('/home/amal/dev/tx/txtx/addons/evm/src/tests/integration')
    
    converted = 0
    skipped = 0
    
    for filepath in test_dir.glob('*.rs'):
        if convert_test_file(filepath):
            converted += 1
        else:
            skipped += 1
    
    print(f"\n📊 Summary: {converted} files converted, {skipped} files skipped")

if __name__ == '__main__':
    main()