#!/usr/bin/env python3
import sys
import re

def fix_file(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # Pattern 1: debug line inside record_event parameters (wrong placement)
        # Look for: runtime_trace::record_event( ... tracing::debug!( ... ) ... );
        # Actually we need to find lines where tracing::debug! appears between
        # runtime_trace::record_event and the closing );
        if 'runtime_trace::record_event' in line:
            start_idx = i
            # Find the closing ); for this call
            # We'll search forward for a line that ends with ');' 
            # with same or less indentation as the opening line
            opening_indent = len(line) - len(line.lstrip())
            j = i
            found_debug = False
            debug_line_idx = -1
            
            while j < len(lines):
                # Check if this line has a debug statement
                if 'tracing::debug!' in lines[j] and j > start_idx:
                    found_debug = True
                    debug_line_idx = j
                    debug_line = lines[j]
                
                # Check for closing ); with same or less indentation
                stripped = lines[j].strip()
                if stripped.endswith(');') and (len(lines[j]) - len(lines[j].lstrip()) <= opening_indent):
                    closing_idx = j
                    
                    if found_debug and debug_line_idx < closing_idx:
                        # The debug line is inside the record_event call
                        # We need to move it outside
                        print(f"Found debug line inside record_event at line {debug_line_idx+1}")
                        print(f"  Opening at line {start_idx+1}, closing at line {closing_idx+1}")
                        
                        # Remove the debug line
                        debug_line_content = lines.pop(debug_line_idx)
                        # Adjust indices since we removed a line
                        if debug_line_idx < closing_idx:
                            closing_idx -= 1
                        
                        # Insert the debug line after the closing );
                        indent = len(lines[closing_idx]) - len(lines[closing_idx].lstrip())
                        lines.insert(closing_idx + 1, ' ' * indent + debug_line_content.lstrip())
                        
                        # Update i to skip over the fixed section
                        i = closing_idx + 1
                        break
                
                j += 1
        
        i += 1
    
    # Pattern 2: Fix unclosed JSON in tool_call_start
    # Find the specific tool_call_start that's missing closing braces
    i = 0
    while i < len(lines):
        if '"tool_call_start"' in lines[i] and 'serde_json::json!({' in lines[i+7]:  # 7 lines after event name
            # Find the arguments line and add closing braces
            j = i + 7  # Start of JSON
            # Look for the line with "arguments": ... 
            while j < len(lines):
                if '"arguments"' in lines[j]:
                    # The next line should be empty or something else
                    # Insert closing braces after this line
                    indent = len(lines[j]) - len(lines[j].lstrip())
                    # Close JSON object and record_event call
                    lines.insert(j + 1, ' ' * (indent - 4) + '}),\n')
                    lines.insert(j + 2, ' ' * (indent - 8) + ');\n')
                    break
                j += 1
            break
        i += 1
    
    # Write back
    with open(file_path, 'w', encoding='utf-8') as f:
        f.writelines(lines)
    print(f"Fixed {file_path}")

if __name__ == '__main__':
    fix_file("./zeroclaw/src/agent/loop_.rs")