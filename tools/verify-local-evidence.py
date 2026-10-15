"""Read-only verification of the locally retained sources named by oracle cases."""

import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("investigation_root", type=Path)
    args = parser.parse_args()
    root = args.investigation_root.resolve(strict=True)
    project = Path(__file__).resolve().parent.parent
    executable = root / "extracted/floppies/WS400-DISK2-PROG/WS.EXE"
    executable_hash = hashlib.sha256(executable.read_bytes()).hexdigest()
    examined = {}
    cases = []
    for path in sorted((project / "tests/oracles").glob("*.json")):
        case = json.loads(path.read_text(encoding="utf-8"))
        if case["provenance"]["executable_sha256"] != executable_hash:
            raise ValueError("Reference executable identity differs")
        for source in case["provenance"]["sources"]:
            target = (root / source["path"]).resolve(strict=True)
            if not target.is_relative_to(root):
                raise ValueError("Source escapes the selected investigation directory")
            actual = hashlib.sha256(target.read_bytes()).hexdigest()
            if actual != source["sha256"]:
                raise ValueError("Source identity differs: " + source["path"])
            examined[source["path"]] = actual
        cases.append(case["id"])
    if not cases:
        raise ValueError("No oracle cases found")
    print(json.dumps({"cases": cases, "verified_source_files": len(examined),
                      "reference_executable_sha256": executable_hash,
                      "target_executed": False, "all_identities_match": True}, indent=2))


if __name__ == "__main__":
    main()
