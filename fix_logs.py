#!/usr/bin/env python3
import sys
import re

def find_closing_paren(lines, start_idx):
    """Find the line index of the closing '),' or ');' for a record_event call."""
    # Simple heuristic: find the next line that starts with the same indentation
    # and ends with '),' or ');'
    base_indent = len(lines[start_idx]) - len(lines[start_idx].lstrip())
    i = start_idx
    while i < len(lines):
        stripped = lines[i].strip()
        if stripped.endswith('),') or stripped.endswith(');'):
            # Ensure this is not inside a nested structure (like json!)
            # Check if the line starts with at least base_indent spaces
            if len(lines[i]) - len(lines[i].lstrip()) == base_indent:
                return i
        i += 1
    return None

def insert_after(lines, idx, new_line):
    """Insert new_line after line idx."""
    indent = len(lines[idx]) - len(lines[idx].lstrip())
    # If the line is empty, keep same indent as previous non-empty line?
    lines.insert(idx + 1, ' ' * indent + new_line + '\n')

def main():
    file_path = "./zeroclaw/src/agent/loop_.rs"
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # 1. LLM request observer event
        if 'observer.record_event(&ObserverEvent::LlmRequest' in line:
            # Find the closing '});' (should be within a few lines)
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('});'):
                    # Insert debug line after this line
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    lines.insert(j + 1, ' ' * indent + 'tracing::debug!("LLM request: iteration={}, provider={}, model={}, messages={}", iteration + 1, provider_name, model, history.len());\n')
                    i = j + 2  # skip inserted line
                    break
                j += 1
        
        # 2. tool_call_start event
        if '"tool_call_start"' in line and 'runtime_trace::record_event' in lines[i-1]:
            # Find the closing '),' after the JSON object
            # Look for a line that ends with '}),' then the next line ends with '),'
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('}),'):
                    # Next line should be '),'
                    if j+1 < len(lines) and lines[j+1].strip() == '),':
                        closing_idx = j+1
                        indent = len(lines[closing_idx]) - len(lines[closing_idx].lstrip())
                        lines.insert(closing_idx + 1, ' ' * indent + 'tracing::debug!("Tool start: {} args={}", tool_name, scrub_credentials(&tool_args.to_string()));\n')
                        i = closing_idx + 2
                        break
                j += 1
        
        # 3. tool_call_result events (after tool execution)
        if '"tool_call_result"' in line and 'runtime_trace::record_event' in lines[i-1]:
            # Check if this is inside the tool execution loop (look for 'outcome' variable in nearby lines)
            # Simpler: find the closing '),' after the JSON object
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('}),'):
                    # Next line should be '),'
                    if j+1 < len(lines) and lines[j+1].strip() == '),':
                        closing_idx = j+1
                        indent = len(lines[closing_idx]) - len(lines[closing_idx].lstrip())
                        lines.insert(closing_idx + 1, ' ' * indent + 'tracing::debug!("Tool result: {} success={} duration={:?} output={}", call.name, outcome.success, outcome.duration, scrub_credentials(&outcome.output));\n')
                        i = closing_idx + 2
                        break
                j += 1
        
        i += 1
    
    # Write back
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    print(f"Fixed logs in {file_path}")

if __name__ == '__main__':
    main()