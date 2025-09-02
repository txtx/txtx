#!/usr/bin/env python3
"""
Script to fix missing contract_abi fields in call_contract actions
"""

import os
import re
from pathlib import Path

def add_missing_abi(filepath):
    """Add contract_abi field to call_contract actions that are missing it"""
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    modified = False
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Check if this is a call_contract action
        if 'action' in line and '"evm::call_contract"' in line:
            # Find the block
            block_start = i
            brace_count = 0
            block_end = i
            
            # Find where the block starts
            for j in range(i, len(lines)):
                if '{' in lines[j]:
                    brace_count += lines[j].count('{')
                    brace_count -= lines[j].count('}')
                    if brace_count > 0:
                        break
            
            # Find where the block ends
            for j in range(i + 1, len(lines)):
                brace_count += lines[j].count('{')
                brace_count -= lines[j].count('}')
                if brace_count == 0:
                    block_end = j
                    break
            
            # Check if contract_abi is present in this block
            block_text = ''.join(lines[block_start:block_end + 1])
            if 'contract_abi' not in block_text:
                # Add a generic ABI after contract_address
                for j in range(block_start + 1, block_end):
                    if 'contract_address' in lines[j]:
                        # Insert contract_abi on the next line
                        indent = len(lines[j]) - len(lines[j].lstrip())
                        abi_line = ' ' * indent + 'contract_abi = action.deploy.contract_abi  # Use deployed contract ABI\n'
                        
                        # Special cases where we know what ABI to use
                        if 'getValue' in block_text:
                            abi_line = ' ' * indent + 'contract_abi = \'[{"inputs":[],"name":"getValue","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"}]\'\n'
                        elif 'setValue' in block_text:
                            abi_line = ' ' * indent + 'contract_abi = \'[{"inputs":[{"internalType":"uint256","name":"value","type":"uint256"}],"name":"setValue","outputs":[],"stateMutability":"nonpayable","type":"function"}]\'\n'
                        elif 'transfer' in block_text:
                            abi_line = ' ' * indent + 'contract_abi = \'[{"inputs":[{"internalType":"address","name":"to","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"transfer","outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"}]\'\n'
                        elif 'action.deploy' in block_text:
                            # If it references a deploy action, use its ABI
                            abi_line = ' ' * indent + 'contract_abi = action.deploy.contract_abi\n'
                        
                        lines.insert(j + 1, abi_line)
                        modified = True
                        print(f"  Added contract_abi for call_contract action at line {j}")
                        i = j + 2  # Skip past the inserted line
                        break
        
        i += 1
    
    if modified:
        with open(filepath, 'w') as f:
            f.writelines(lines)
        return True
    return False

def main():
    """Fix all runbook files with missing contract_abi"""
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
            # Check if file has call_contract
            with open(tx_file, 'r') as f:
                content = f.read()
            
            if 'evm::call_contract' in content:
                total_count += 1
                relative_path = tx_file.relative_to(base_dir)
                
                if add_missing_abi(tx_file):
                    fixed_count += 1
                    print(f"  ✅ Fixed: {relative_path}")
                else:
                    if 'contract_abi' not in content:
                        print(f"  ⚠️  Manual review needed: {relative_path}")
                    else:
                        print(f"  ⏭️  Already has contract_abi: {relative_path}")
    
    print(f"\n📊 Summary:")
    print(f"  Total files with call_contract: {total_count}")
    print(f"  Files fixed: {fixed_count}")

if __name__ == '__main__':
    main()