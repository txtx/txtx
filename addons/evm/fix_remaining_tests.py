#!/usr/bin/env python3
"""Fix remaining tests that have mixed old/new code"""

import re
from pathlib import Path

files_to_fix = [
    'check_confirmations_tests.rs',
    'basic_execution_test.rs',
    'contract_interaction_tests.rs', 
    'debug_unsupervised_test.rs',
    'error_handling_tests.rs',
    'test_confirmations_issue.rs',
    'test_state_reading.rs',
    'test_structured_logs.rs',
]

def fix_file(filepath):
    """Fix a file with mixed patterns"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # Remove any remaining ProjectTestHarness imports
    content = re.sub(
        r'use crate::tests::test_harness::\{?ProjectTestHarness\}?;?\n',
        '',
        content
    )
    
    # If MigrationHelper not imported, add it
    if 'use crate::tests::fixture_builder' not in content:
        # Add after the mod declaration
        content = re.sub(
            r'(mod \w+ \{)',
            r'\1\n    use crate::tests::fixture_builder::MigrationHelper;',
            content
        )
    
    # Fix any remaining ProjectTestHarness usage patterns
    
    # Pattern: ProjectTestHarness::from_fixture(&fixture_path)
    content = re.sub(
        r'ProjectTestHarness::from_fixture\(&fixture_path\)',
        r'MigrationHelper::from_fixture(&fixture_path)',
        content
    )
    
    # Pattern: mut harness = 
    content = re.sub(
        r'let mut harness = MigrationHelper',
        r'let harness = MigrationHelper',
        content
    )
    
    # Pattern: .with_anvil() - remove it
    content = re.sub(
        r'\.with_anvil\(\)',
        '',
        content
    )
    
    # Fix execute patterns - should be .execute().await
    content = re.sub(
        r'let result = harness\s*\n?\s*\.execute\(\)\s*\n?\s*\.await',
        r'let result = harness.execute().await',
        content
    )
    
    # Special pattern for new_with_content
    if 'ProjectTestHarness::new_with_content' in content:
        # This needs special handling - convert to FixtureBuilder
        content = re.sub(
            r'use crate::tests::test_harness::.*?;',
            'use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};',
            content
        )
        
        # Convert the new_with_content pattern
        content = re.sub(
            r'let harness = ProjectTestHarness::new_with_content\(\s*"([^"]+)",\s*([^)]+)\s*\)\.with_anvil\(\);',
            r'let fixture = FixtureBuilder::new("\1")\n            .with_runbook("main", \2)\n            .build()\n            .await\n            .expect("Failed to build fixture");',
            content
        )
        
        # Convert setup() calls to execute
        content = re.sub(
            r'harness\.setup\(\)\.expect\([^)]+\);',
            r'// Project already set up by FixtureBuilder',
            content
        )
        
        # Fix path references
        content = re.sub(
            r'harness\.project_path',
            r'fixture.project_dir',
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
    for filename in files_to_fix:
        filepath = test_dir / filename
        if filepath.exists():
            if fix_file(filepath):
                fixed += 1
    
    print(f"\n📊 Fixed {fixed} files")

if __name__ == '__main__':
    main()