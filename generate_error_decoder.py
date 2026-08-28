import re

with open("contracts/predictify-hybrid/src/err.rs", "r") as f:
    content = f.read()

enum_match = re.search(r"pub enum Error \{([^}]*)\}", content)
if not enum_match:
    print("Could not find Error enum")
    exit(1)

variants = []
for line in enum_match.group(1).split('\n'):
    m = re.match(r"\s+([A-Za-z0-9_]+)\s*=\s*([0-9]+),", line)
    if m:
        variants.append((m.group(1), int(m.group(2))))

impl_str = """
impl Error {
    /// Safely decodes a u32 into an Error variant.
    /// Returns `Error::UnknownError` if the code is not recognized.
    pub fn decode(code: u32) -> Self {
        match code {
"""
for name, code in variants:
    impl_str += f"            {code} => Error::{name},\n"

impl_str += """            _ => Error::UnknownError,
        }
    }
}
"""
print(impl_str)
