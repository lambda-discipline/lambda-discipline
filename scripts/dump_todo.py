"""
This script recursively scans the current directory for lines containing "TOD0:" and
prints them in the format relative_filepath:line_number: TOD0: tod0 text.
It skips scanning itself, respects .gitignore rules using the pathspec library, and
ignores files that cannot be read due to encoding or permission issues.
P.S. TOD0 is intentionally misspelled here, and in above description.
"""

import os
import pathspec

BASE_DIR = "."
SCRIPT_PATH = os.path.abspath(__file__)  # Absolute path of this script

EXCLUDE_PATHS = {
    os.path.relpath(SCRIPT_PATH, BASE_DIR),
    ".git",
}

GITIGNORE_PATH = os.path.join(BASE_DIR, ".gitignore")
spec = None
if os.path.exists(GITIGNORE_PATH):
    with open(GITIGNORE_PATH, "r", encoding="utf-8") as f:
        spec = pathspec.PathSpec.from_lines("gitwildmatch", f)

for root, _, files in os.walk(BASE_DIR):
    for filename in files:
        filepath: str = os.path.join(root, filename)
        rel_path: str = os.path.relpath(filepath, BASE_DIR)
        if any(rel_path == p or rel_path.startswith(p + os.sep) for p in EXCLUDE_PATHS):
            continue
        if spec and spec.match_file(rel_path):
            continue
        try:
            with open(filepath, "r", encoding="utf-8") as f:
                for lineno, line in enumerate(f, start=1):
                    if "TODO:" in line:
                        todo_index = line.index("TODO:")
                        todo_text = line[todo_index:].strip()
                        print(f"{rel_path}:{lineno}: {todo_text}")
        except (UnicodeDecodeError, PermissionError):
            # Skip files that can’t be read as text
            continue
