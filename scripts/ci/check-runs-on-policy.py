#!/usr/bin/env python3
"""Guardrail: keep workflow runner selectors within the leafchains policy."""

from __future__ import annotations

import glob
import sys
from pathlib import Path
from typing import Iterable

try:
    import yaml
except ModuleNotFoundError:  # pragma: no cover
    yaml = None

ALLOWED_STRINGS = {"hetzner-thxnet", "ubuntu-latest"}
WORKFLOW_GLOBS = [".github/workflows/*.yml", ".github/workflows/*.yaml"]


def _iter_workflows() -> Iterable[Path]:
    for g in WORKFLOW_GLOBS:
        for p in sorted(glob.glob(g)):
            yield Path(p)


def main() -> int:
    if yaml is None:
        print("pyyaml not available; skipping strict workflow check", file=sys.stderr)
        return 0

    errors = []
    for path in _iter_workflows():
        data = yaml.safe_load(path.read_text()) or {}
        jobs = data.get("jobs") or {}

        for job_name, job in jobs.items():
            if not isinstance(job, dict):
                continue

            runs_on = job.get("runs-on")
            if runs_on is None:
                continue

            if isinstance(runs_on, str):
                if runs_on not in ALLOWED_STRINGS:
                    errors.append(f"{path}:{job_name} runs-on='{runs_on}' is not in allowed set {sorted(ALLOWED_STRINGS)}")
            elif isinstance(runs_on, list):
                errors.append(f"{path}:{job_name} runs-on is list {runs_on}; convert to single policy label")
            else:
                errors.append(f"{path}:{job_name} runs-on type {type(runs_on).__name__} unsupported: {runs_on!r}")

    if errors:
        print("Runner selector policy check failed:")
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print("Runner selector policy check passed: all workflow jobs use allowed selector set.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
