"""Lists the keyed nodes of the accessibility tree in a Rune `--tree` dump:
key, role, name, states and actions.

    python3 tools/tree.py tree.json
"""

import json
import sys


def walk(node, depth=0):
    if node.get("key"):
        print(
            f'{"  " * depth}{node["key"]:<14} role={node.get("role"):<8} '
            f'name={node.get("name")!r:<24} states={node.get("states", [])} '
            f'actions={node.get("actions", [])}'
        )
    for child in node.get("children", []):
        walk(child, depth + (1 if node.get("key") else 0))


tree = json.load(open(sys.argv[1]))
print("t:", tree["t"], "warnings:", tree.get("warnings"))
walk(tree["accessibility"])
