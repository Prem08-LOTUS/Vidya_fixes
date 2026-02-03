import os

def gather_code():
    extensions = ['.rs', '.py', '.toml']
    root_dirs = ['agni-os/src', 'metrology', 'hardware', 'agni-os']

    output = ""

    for root_dir in root_dirs:
        for dirpath, dirnames, filenames in os.walk(root_dir):
            # Skip target or hidden dirs just in case
            if 'target' in dirpath or '.git' in dirpath:
                continue

            for f in filenames:
                if any(f.endswith(ext) for ext in extensions):
                    # specific exclusion of massive lock files if any, though .toml is usually small
                    filepath = os.path.join(dirpath, f)
                    try:
                        with open(filepath, 'r', encoding='utf-8') as file:
                            content = file.read()
                            output += f"\n\n--- START FILE: {filepath} ---\n"
                            output += content
                            output += f"\n--- END FILE: {filepath} ---\n"
                    except Exception as e:
                        print(f"Skipping {filepath}: {e}")

    with open('codebase_dump.txt', 'w', encoding='utf-8') as f:
        f.write(output)
    print(f"Gathered {len(output)} bytes of code.")

if __name__ == "__main__":
    gather_code()
