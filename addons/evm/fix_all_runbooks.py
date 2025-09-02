#!/usr/bin/env python3
"""
Script to automatically fix common field name issues in runbook files
"""

import os
import re
from pathlib import Path

# Define replacements for each action type
FIELD_REPLACEMENTS = {
    'send_eth': {
        r'\bto\s*=': 'recipient_address =',
        r'\bvalue\s*=': 'amount =',
        r'\bfrom\s*=': '# from field removed - using signer\n    # from =',
    },
    'call_contract': {
        r'\bcontract\s*=': 'contract_address =',
        r'\babi\s*=': 'contract_abi =',
        r'\bfunction\s*=': 'function_name =',
        r'\bfunction_arguments\s*=': 'function_args =',
    },
    'deploy_contract': {
        r'\babi\s*=': 'contract_abi =',
        r'\bbytecode\s*=': 'contract_bytecode =',
        r'\bconstructor_arguments\s*=': 'constructor_args =',
    },
    'eth_call': {
        r'\bcontract\s*=': 'contract_address =',
        r'\babi\s*=': 'contract_abi =',
        r'\bfunction\s*=': 'function_name =',
        r'\bfunction_arguments\s*=': 'function_args =',
    },
}

# Also fix signer types
SIGNER_REPLACEMENTS = {
    r'signer\s+"([^"]+)"\s+"evm::private_key"': r'signer "\1" "evm::secret_key"',
    r"signer\s+'([^']+)'\s+'evm::private_key'": r"signer '\1' 'evm::secret_key'",
}

def fix_runbook_file(filepath):
    """Fix field names in a single runbook file"""
    with open(filepath, 'r') as f:
        content = f.read()
    
    original_content = content
    
    # Find all actions in the file
    action_pattern = r'action\s+"[^"]+"\s+"(\w+)::(\w+)"'
    actions = re.findall(action_pattern, content)
    
    # Process each action type found
    for namespace, action_type in actions:
        if namespace != 'evm':
            continue
            
        if action_type in FIELD_REPLACEMENTS:
            # Find the action block
            action_block_pattern = (
                rf'(action\s+"[^"]+"\s+"{namespace}::{action_type}"\s*\{{[^}}]*\}})'
            )
            
            def replace_in_block(match):
                block = match.group(1)
                for pattern, replacement in FIELD_REPLACEMENTS[action_type].items():
                    # Only replace within this specific action block
                    block = re.sub(pattern, replacement, block)
                return block
            
            content = re.sub(action_block_pattern, replace_in_block, content, flags=re.DOTALL)
    
    # Fix signer types
    for pattern, replacement in SIGNER_REPLACEMENTS.items():
        content = re.sub(pattern, replacement, content)
    
    # Remove quotes from numeric values
    # Match patterns like: field = "12345" or field = '12345'
    content = re.sub(
        r'(\w+\s*=\s*)["\'](\d+)["\']',
        r'\1\2',
        content
    )
    
    # Special case for wei values with underscores
    content = re.sub(
        r'(\w+\s*=\s*)["\'](\d+(?:_\d+)*)["\']',
        r'\1\2',
        content
    )
    
    if content != original_content:
        with open(filepath, 'w') as f:
            f.write(content)
        return True
    return False

def main():
    """Fix all runbook files in the fixtures directory"""
    base_dir = Path(__file__).parent
    fixtures_dirs = [
        base_dir / 'fixtures',
        base_dir / 'src' / 'tests' / 'fixtures',
    ]
    
    fixed_count = 0
    total_count = 0
    
    for fixtures_dir in fixtures_dirs:
        if not fixtures_dir.exists():
            continue
            
        print(f"📁 Processing {fixtures_dir}")
        
        for tx_file in fixtures_dir.rglob('*.tx'):
            total_count += 1
            relative_path = tx_file.relative_to(base_dir)
            
            if fix_runbook_file(tx_file):
                fixed_count += 1
                print(f"  ✅ Fixed: {relative_path}")
            else:
                print(f"  ⏭️  No changes needed: {relative_path}")
    
    print(f"\n📊 Summary:")
    print(f"  Total files processed: {total_count}")
    print(f"  Files fixed: {fixed_count}")
    print(f"  Files already correct: {total_count - fixed_count}")

if __name__ == '__main__':
    main()