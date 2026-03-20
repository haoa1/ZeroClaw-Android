#!/usr/bin/env python3
import sys
import re

def add_logs(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    # 1. Add debug after LLM request observer event
    i = 0
    while i < len(lines):
        if 'observer.record_event(&ObserverEvent::LlmRequest' in lines[i]:
            # Find the closing '});' for this observer call
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('});'):
                    # Insert debug line after this line
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    lines.insert(j + 1, ' ' * indent + 'tracing::debug!("LLM request: iteration={}, provider={}, model={}, messages={}", iteration + 1, provider_name, model, history.len());\n')
                    i = j + 2  # Skip inserted line
                    break
                j += 1
        i += 1
    
    # 2. Add debug after tool_call_start event
    i = 0
    while i < len(lines):
        if '"tool_call_start"' in lines[i]:
            # This is inside a runtime_trace::record_event call
            # Find the closing '),' or ');' for this record_event call
            # First, find the start of the record_event call (look backward for 'runtime_trace::record_event(')
            start_idx = i
            while start_idx >= 0 and 'runtime_trace::record_event' not in lines[start_idx]:
                start_idx -= 1
            
            if start_idx < 0:
                i += 1
                continue
            
            # Find the closing parenthesis after the JSON object
            # Look for a line with '}),' then the next line should be '),'
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('}),'):
                    # Next line should be the closing '),'
                    if j + 1 < len(lines) and lines[j + 1].strip() == '),':
                        closing_idx = j + 1
                        # Insert debug line after the closing parenthesis
                        indent = len(lines[closing_idx]) - len(lines[closing_idx].lstrip())
                        # We need to extract tool_name variable which should be in scope
                        lines.insert(closing_idx + 1, ' ' * indent + 'tracing::debug!("Tool start: {} args={}", tool_name, scrub_credentials(&tool_args.to_string()));\n')
                        i = closing_idx + 2
                        break
                j += 1
        i += 1
    
    # 3. Add debug after tool_call_result events
    # We need to find all tool_call_result events in the tool execution loop
    # They appear in multiple places (cancelled, denied, duplicate, normal execution)
    # We'll handle them similarly to tool_call_start
    i = 0
    while i < len(lines):
        if '"tool_call_result"' in lines[i]:
            # Find the closing parenthesis
            # Look for '}),' then '),'
            j = i
            while j < len(lines):
                if lines[j].strip().endswith('}),'):
                    # Check if next line is '),'
                    if j + 1 < len(lines) and lines[j + 1].strip() == '),':
                        closing_idx = j + 1
                        # Insert debug line after
                        indent = len(lines[closing_idx]) - len(lines[closing_idx].lstrip())
                        # Determine which variables are in scope
                        # In most cases, 'call.name' and 'outcome' should be available
                        # but for cancelled/denied/duplicate cases, the outcome is constructed inline
                        # Let's check context to see if 'outcome' variable exists
                        # For simplicity, we'll add a generic log that works for all
                        # We need to examine the surrounding code to know what variables are available
                        
                        # Actually, let's skip for now and handle manually
                        # We'll come back to this
                        pass
                        i = closing_idx + 1
                        break
                j += 1
        i += 1
    
    # Write back
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    print(f"Added logs to {file_path}")

if __name__ == '__main__':
    add_logs("./zeroclaw/src/agent/loop_.rs")