"""Suite loader: validates the benchmark task fixtures against the spec's
integrity invariants (§1) before the harness will run them.

Every task directory must carry:
  - `task.md`         (a human-written problem statement + `language:` header)
  - `baseline/`       (non-empty; the pre-verified failing starting point)
  - `tests/trusted/`  (at least one non-empty test file the model may see)
  - `tests/held_out/` (at least one non-empty test file neither method may see)

Any missing/empty piece is a `SuiteIntegrityError` naming every offending task
in one pass, so a broken suite yields a single actionable message rather than
one failure per reload.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


REQUIRED_LANGUAGES = {"python": 10, "csharp": 10}
REQUIRED_TOTAL_TASKS = 30


class SuiteIntegrityError(Exception):
    """Raised when a suite fails one or more of the spec's integrity invariants."""


@dataclass(frozen=True)
class Task:
    id: str
    language: str
    path: Path


@dataclass(frozen=True)
class Suite:
    tasks: tuple[Task, ...]

    def by_language(self) -> dict[str, int]:
        counts: dict[str, int] = {}
        for t in self.tasks:
            counts[t.language] = counts.get(t.language, 0) + 1
        return counts


def _read_language(task_md: Path) -> str | None:
    """Pulls the `language: <name>` header line from a task.md file."""
    try:
        for line in task_md.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("language:"):
                return stripped.split(":", 1)[1].strip().lower() or None
    except OSError:
        return None
    return None


def _dir_has_content(path: Path) -> bool:
    if not path.is_dir():
        return False
    return any(p.is_file() and p.stat().st_size > 0 for p in path.rglob("*"))


def _any_file_nonempty(path: Path) -> bool:
    if not path.is_dir():
        return False
    return any(p.is_file() and p.stat().st_size > 0 for p in path.rglob("*"))


def load_suite(tasks_root: Path) -> Suite:
    """Loads and validates every task under `tasks_root`, then enforces the
    spec's aggregate invariants (>=30 tasks total; >=10 python and >=10 csharp).
    Raises `SuiteIntegrityError` collecting every offending task id."""

    if not tasks_root.is_dir():
        raise SuiteIntegrityError(f"tasks root {tasks_root!s} is not a directory")

    tasks: list[Task] = []
    errors: list[str] = []

    for entry in sorted(tasks_root.iterdir()):
        if not entry.is_dir():
            continue
        tid = entry.name

        task_md = entry / "task.md"
        if not task_md.is_file() or task_md.stat().st_size == 0:
            errors.append(f"{tid}: missing or empty task.md")
            continue

        language = _read_language(task_md)
        if not language:
            errors.append(f"{tid}: task.md missing `language:` header")
            continue

        baseline = entry / "baseline"
        if not _dir_has_content(baseline):
            errors.append(f"{tid}: baseline/ is empty or missing")
            continue

        trusted = entry / "tests" / "trusted"
        if not _any_file_nonempty(trusted):
            errors.append(f"{tid}: tests/trusted has no non-empty file")
            continue

        held_out = entry / "tests" / "held_out"
        if not _any_file_nonempty(held_out):
            errors.append(f"{tid}: tests/held_out has no non-empty file")
            continue

        tasks.append(Task(id=tid, language=language, path=entry))

    if errors:
        raise SuiteIntegrityError("suite integrity failed:\n  " + "\n  ".join(errors))

    if len(tasks) < REQUIRED_TOTAL_TASKS:
        raise SuiteIntegrityError(
            f"suite has {len(tasks)} tasks, must be >= {REQUIRED_TOTAL_TASKS}"
        )

    counts: dict[str, int] = {}
    for t in tasks:
        counts[t.language] = counts.get(t.language, 0) + 1
    for lang, minimum in REQUIRED_LANGUAGES.items():
        got = counts.get(lang, 0)
        if got < minimum:
            raise SuiteIntegrityError(
                f"suite has {got} {lang} tasks, must be >= {minimum}"
            )

    return Suite(tasks=tuple(tasks))
