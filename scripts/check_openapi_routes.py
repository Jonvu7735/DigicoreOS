#!/usr/bin/env python3
"""OpenAPI contract guard.

Every route a service actually serves under /api/v1 must be documented in
docs/openapi.yaml. This is ONE-directional (served ⊆ documented): the spec may
also document not-yet-implemented routes (it's the forward contract), but it must
never omit a route that already exists — otherwise the generated client and any
consumer are blind to it.

Pure stdlib (no PyYAML) so CI needs no extra install. Run from the repo root:

    python3 scripts/check_openapi_routes.py
"""
import glob
import re
import sys

# Infra endpoints are intentionally not part of the business contract.
INFRA = {"/health", "/ready", "/metrics"}


def served_routes():
    """(file, full_path) for every business route a service mounts."""
    routes = set()
    for f in sorted(
        glob.glob("services/*/src/api/http/routes.rs")
        + glob.glob("verticals/*/src/api/http/routes.rs")
    ):
        text = open(f).read()
        nest = re.search(r'\.nest\("([^"]+)"', text)
        if not nest:
            continue
        prefix = nest.group(1)
        # `\s*` so multi-line `.route(\n  "/path",\n  ...)` (rustfmt-wrapped) is caught.
        for sub in re.findall(r'\.route\(\s*"([^"]+)"', text):
            if sub in INFRA:
                continue
            routes.add((f, prefix + sub))
    return routes


def documented_paths():
    """Top-level path keys in docs/openapi.yaml (between `paths:` and `components:`)."""
    text = open("docs/openapi.yaml").read()
    segment = text.split("\npaths:", 1)[1].split("\ncomponents:", 1)[0]
    return set(re.findall(r"^  (/\S+):\s*$", segment, re.M))


def main():
    documented = documented_paths()
    served = served_routes()
    violations = sorted((path, f) for (f, path) in served if path not in documented)

    print(f"served business routes: {len(served)} | documented paths: {len(documented)}")
    if violations:
        print(f"\nERROR: {len(violations)} served route(s) missing from docs/openapi.yaml:")
        for path, f in violations:
            print(f"  {path}   (served by {f})")
        print("\nDocument them in docs/openapi.yaml, then regenerate the TS client")
        print("(cd clients/typescript && npm run generate).")
        sys.exit(1)
    print("OK: every served /api/v1 route is documented.")


if __name__ == "__main__":
    main()
