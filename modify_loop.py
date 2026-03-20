#!/usr/bin/env python3
import sys
import re

def main():
    file_path = "./zeroclaw/src/agent/loop_.rs"
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    new_lines = []
    i = 0
    total_lines = len(lines)
    
    while i < total_lines:
        line = lines[i]
        new_lines.append(line)
        
        # 1. After LLM request observer event
        if 'observer.record_event(&ObserverEvent::LlmRequest' in line:
            # Find the closing brace of this call (should be a few lines later)
            # We'll insert after the line that contains '});' for the observer call.
            # Actually the pattern is observer.record_event(...); so we need to find the semicolon.
            # Let's look ahead up to 5 lines for a line ending with ');'
            j = i
            while j < min(i+10, total_lines):
                if lines[j].strip().endswith('});'):
                    # Insert debug log after this line
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    new_lines.append(' ' * indent + 'tracing::debug!("LLM request: iteration={}, provider={}, model={}, messages={}", iteration + 1, provider_name, model, history.len());\n')
                    break
                j += 1
        
        # 2. After tool_call_start event
        if '"tool_call_start"' in line:
            # Find the closing parenthesis of the record_event call
            # Look ahead for a line that ends with '),' or ');'
            j = i
            while j < min(i+15, total_lines):
                if lines[j].strip().endswith('),') or lines[j].strip().endswith(');'):
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    # We need to extract tool_name variable; it's in the JSON payload.
                    # We'll just log with placeholders that will be in scope.
                    new_lines.append(' ' * indent + 'tracing::debug!("Tool start: {} args={}", tool_name, scrub_credentials(&tool_args.to_string()));\n')
                    break
                j += 1
        
        # 3. After tool_call_result event (for each tool execution)
        if '"tool_call_result"' in line and 'runtime_trace::record_event' in lines[i-1]:
            # Find the closing parenthesis
            j = i
            while j < min(i+20, total_lines):
                if lines[j].strip().endswith('),') or lines[j].strip().endswith(');'):
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    new_lines.append(' ' * indent + 'tracing::debug!("Tool result: {} success={} duration={:?} output={}", call.name, outcome.success, outcome.duration, scrub_credentials(&outcome.output));\n')
                    break
                j += 1
        
        i += 1
    
    # Write back
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(new_lines)
    print(f"Modified {file_path}")

if __name__ == '__main__':
    main()