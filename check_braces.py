import sys

def check_braces(filename):
    with open(filename, 'r') as f:
        content = f.read()
    
    stack = []
    for i, char in enumerate(content):
        if char == '{':
            stack.append(i)
        elif char == '}':
            if not stack:
                print(f"Extra closing brace at index {i}")
            else:
                stack.pop()
    
    for pos in stack:
        print(f"Unclosed opening brace at index {pos}")
        # Find line number
        line_no = content.count('\n', 0, pos) + 1
        print(f"  Line: {line_no}")
        print(f"  Context: {content[pos:pos+50]}...")

if __name__ == "__main__":
    check_braces(sys.argv[1])
