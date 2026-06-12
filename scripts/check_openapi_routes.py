#!/usr/bin/env python3
"""OpenAPI contract guard.

The business routes served under /api/v1 must match docs/openapi.yaml EXACTLY
(served == documented), checked both ways:
  - served ⊄ documented -> a route exists with no spec entry, so the generated
    client and any consumer are blind to it;
  - documented ⊄ served -> the spec promises an endpoint no service implements.
Keeping the two equal stops the spec drifting ahead of (or behind) the code.

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
    served_pairs = served_routes()
    served = {path for (_f, path) in served_pairs}

    undocumented = sorted((path, f) for (f, path) in served_pairs if path not in documented)
    unserved = sorted(documented - served)

    print(f"served business routes: {len(served_pairs)} | documented paths: {len(documented)}")
    ok = True
    if undocumented:
        ok = False
        print(f"\nERROR: {len(undocumented)} served route(s) missing from docs/openapi.yaml:")
        for path, f in undocumented:
            print(f"  {path}   (served by {f})")
        print("\nDocument them in docs/openapi.yaml, then regenerate the TS client")
        print("(cd clients/typescript && npm run generate).")
    if unserved:
        ok = False
        print(f"\nERROR: {len(unserved)} documented path(s) not served by any route:")
        for path in unserved:
            print(f"  {path}")
        print("\nImplement them (route + handler), or remove them from docs/openapi.yaml.")
    if not ok:
        sys.exit(1)
    print("OK: served /api/v1 routes and documented paths match exactly.")


if __name__ == "__main__":
    main()
