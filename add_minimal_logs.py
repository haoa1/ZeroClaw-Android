#!/usr/bin/env python3
"""
Add minimal debug logs to loop_.rs without breaking syntax.
We add three debug statements:
1. After LLM request observer event
2. After tool_call_start event  
3. After tool_call_result events (in the main execution loop)
"""
import sys
import os

def add_llm_request_debug(lines):
    """Add debug after LLM request observer event."""
    for i, line in enumerate(lines):
        if 'observer.record_event(&ObserverEvent::LlmRequest' in line:
            # Find the closing });
            for j in range(i, min(i + 10, len(lines))):
                if lines[j].strip().endswith('});'):
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    lines.insert(j + 1, ' ' * indent + 'tracing::debug!("LLM request: iteration={}, provider={}, model={}, messages={}", iteration + 1, provider_name, model, history.len());\n')
                    return True
    return False

def add_tool_start_debug(lines):
    """Add debug after tool_call_start event."""
    for i, line in enumerate(lines):
        if '"tool_call_start"' in line and 'runtime_trace::record_event' in lines[i-1]:
            # Find the closing ), after the JSON object
            # Look for '}),' then '),'
            for j in range(i, min(i + 20, len(lines))):
                if lines[j].strip().endswith('}),'):
                    if j + 1 < len(lines) and lines[j + 1].strip() == '),':
                        indent = len(lines[j + 1]) - len(lines[j + 1].lstrip())
                        lines.insert(j + 2, ' ' * indent + 'tracing::debug!("Tool start: {} args={}", tool_name, scrub_credentials(&tool_args.to_string()));\n')
                        return True
    return False

def add_tool_result_debug(lines):
    """Add debug after tool_call_result event in the main execution loop.
    We target the one after tool execution (not cancelled/denied/duplicate)."""
    # Look for the pattern where we have outcome variable (after tool execution)
    for i, line in enumerate(lines):
        if '"tool_call_result"' in line and 'runtime_trace::record_event' in lines[i-1]:
            # Check if this is in the main loop by looking for 'outcome' variable nearby
            # Scan a few lines ahead for 'outcome.'
            for j in range(i, min(i + 15, len(lines))):
                if 'outcome.' in lines[j] or 'outcome.success' in lines[j]:
                    # This is likely the main execution path
                    # Find closing ), after JSON
                    for k in range(i, min(i + 25, len(lines))):
                        if lines[k].strip().endswith('}),'):
                            if k + 1 < len(lines) and lines[k + 1].strip() == '),':
                                indent = len(lines[k + 1]) - len(lines[k + 1].lstrip())
                                lines.insert(k + 2, ' ' * indent + 'tracing::debug!("Tool result: {} success={} duration={:?} output_len={}", call.name, outcome.success, outcome.duration, outcome.output.len());\n')
                                return True
    return False

def main():
    file_path = "./zeroclaw/src/agent/loop_.rs"
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    added_llm = add_llm_request_debug(lines)
    added_start = add_tool_start_debug(lines)
    added_result = add_tool_result_debug(lines)
    
    print(f"Added LLM debug: {added_llm}")
    print(f"Added tool start debug: {added_start}")
    print(f"Added tool result debug: {added_result}")
    
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    
    print(f"Updated {file_path}")

if __name__ == '__main__':
    main()