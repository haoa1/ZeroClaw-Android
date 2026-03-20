#!/usr/bin/env python3
import sys

with open('./zeroclaw/src/agent/loop_.rs', 'r') as f:
    lines = f.readlines()

# Find tool_call_start
for i, line in enumerate(lines):
    if '"tool_call_start"' in line:
        print(f'tool_call_start at line {i+1}')
        # Find closing
        for j in range(i, min(i+30, len(lines))):
            if 'scrub_credentials(&tool_args.to_string())' in line:
                pass
            if lines[j].strip().endswith('}),'):
                print(f'  JSON close at {j+1}: {lines[j].strip()}')
                if j+1 < len(lines):
                    print(f'  Next line {j+2}: {lines[j+1].strip()}')
                    if lines[j+1].strip().endswith('),'):
                        print(f'  Closing at line {j+2}')
                        print(f'  Indent: {len(lines[j+1]) - len(lines[j+1].lstrip())} spaces')
                        # Show context
                        for k in range(max(0, j-5), min(len(lines), j+5)):
                            print(f'{k+1:4}: {lines[k].rstrip()}')
                        break
        break

# Find main tool_call_result
print('\nSearching for main tool_call_result...')
for i, line in enumerate(lines):
    if '"tool_call_result"' in line:
        # Check if near outcome variable
        for j in range(i, min(i+10, len(lines))):
            if 'outcome.success' in lines[j] or 'outcome.output' in lines[j]:
                print(f'Found tool_call_result with outcome at line {i+1}')
                # Find closing
                for k in range(i, min(i+30, len(lines))):
                    if lines[k].strip().endswith('}),'):
                        if k+1 < len(lines) and lines[k+1].strip().endswith('),'):
                            print(f'  Closing at line {k+2}')
                            print(f'  Indent: {len(lines[k+1]) - len(lines[k+1].lstrip())} spaces')
                            # Show context
                            for m in range(max(0, k-5), min(len(lines), k+5)):
                                print(f'{m+1:4}: {lines[m].rstrip()}')
                            break
                break