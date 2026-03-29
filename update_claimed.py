import re
import sys

def update_claimed_checks(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Pattern 1: .claimed.get(X).unwrap_or(false) where we want positive
    # Replace with: .claimed.get(X).map(|info| info.is_claimed()).unwrap_or(false)
    pattern1 = r'\.claimed\.get\(([^)]+)\)\.unwrap_or\(false\)'
    replacement1 = r'.claimed.get(\1).map(|info| info.is_claimed()).unwrap_or(false)'
    
    content = re.sub(pattern1, replacement1, content)
    
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    
    print(f"Updated {file_path}")

if __name__ == '__main__':
    if len(sys.argv) > 1:
        update_claimed_checks(sys.argv[1])
    else:
        print("Usage: python update_claimed.py <file_path>")
