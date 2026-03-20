#!/usr/bin/env python3
import sys

def find_tool_call_start(lines):
    """Find the tool_call_start record_event call and return line index of closing '),'"""
    for i, line in enumerate(lines):
        if '"tool_call_start"' in line:
            # Find the closing '),' after the JSON object
            # Look for '}),' then '),'
            for j in range(i, min(i + 30, len(lines))):
                if lines[j].strip().endswith('}),'):
                    if j + 1 < len(lines) and lines[j + 1].strip() == '),':
                        return j + 1  # Line index of closing '),'
    return -1

def find_main_tool_call_result(lines):
    """Find the main tool_call_result record_event call (after tool execution)"""
    # Look for tool_call_result where there's an outcome variable nearby
    for i, line in enumerate(lines):
        if '"tool_call_result"' in line:
            # Check if this is in the main execution path by looking ahead for 'outcome.'
            for j in range(i, min(i + 20, len(lines))):
                if 'outcome.' in lines[j] or 'outcome.success' in lines[j]:
                    # Find closing '),'
                    for k in range(i, min(i + 30, len(lines))):
                        if lines[k].strip().endswith('}),'):
                            if k + 1 < len(lines) and lines[k + 1].strip() == '),':
                                return k + 1
    return -1

def main():
    file_path = "./zeroclaw/src/agent/loop_.rs"
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # Add tool start debug
    start_close_idx = find_tool_call_start(lines)
    if start_close_idx != -1:
        indent = len(lines[start_close_idx]) - len(lines[start_close_idx].lstrip())
        lines.insert(start_close_idx + 1, ' ' * indent + 'tracing::debug!("Tool start: {} args={}", tool_name, scrub_credentials(&tool_args.to_string()));\n')
        print(f"Added tool start debug after line {start_close_idx + 1}")
    else:
        print("Could not find tool_call_start closing")
    
    # Add tool result debug (main execution)
    result_close_idx = find_main_tool_call_result(lines)
    if result_close_idx != -1:
        indent = len(lines[result_close_idx]) - len(lines[result_close_idx].lstrip())
        lines.insert(result_close_idx + 1, ' ' * indent + 'tracing::debug!("Tool result: {} success={} duration={:?} output_len={}", call.name, outcome.success, outcome.duration, outcome.output.len());\n')
        print(f"Added tool result debug after line {result_close_idx + 1}")
    else:
        print("Could not find main tool_call_result closing")
    
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    print("Updated file")

if __name__ == '__main__':
    main()