"""Static guards for native commands executed by PowerShell workflows."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
NATIVE = r"(?:cargo|python|uv|rustup|node|npm|npx|wrangler)"
NATIVE_INVOCATION = re.compile(
    rf"^(?:&\s*)?{NATIVE}(?:\.exe)?\s|^\$[A-Za-z_]\w*\s*=\s*\(\s*{NATIVE}(?:\.exe)?\s",
    re.IGNORECASE,
)
EXIT_GUARD = re.compile(
    r'^if \(\$LASTEXITCODE -ne 0\) \{ throw ".+ \$LASTEXITCODE" \}$'
)


def literal_run_blocks(text: str) -> list[tuple[int, list[str]]]:
    lines = text.splitlines()
    blocks: list[tuple[int, list[str]]] = []
    for index, line in enumerate(lines):
        match = re.match(r"^(\s*)run:\s*\|\s*$", line)
        if not match:
            continue
        indent = len(match.group(1))
        body: list[str] = []
        cursor = index + 1
        while cursor < len(lines):
            candidate = lines[cursor]
            if candidate.strip() and len(candidate) - len(candidate.lstrip()) <= indent:
                break
            body.append(candidate.strip())
            cursor += 1
        blocks.append((index + 1, body))
    return blocks


def native_failure_guard_issues(text: str) -> list[str]:
    issues: list[str] = []
    for start_line, block in literal_run_blocks(text):
        for index, line in enumerate(block):
            if not NATIVE_INVOCATION.search(line):
                continue
            end = index
            while block[end].endswith("`") and end + 1 < len(block):
                end += 1
            next_index = end + 1
            while next_index < len(block) and not block[next_index]:
                next_index += 1
            if next_index >= len(block) or not EXIT_GUARD.fullmatch(block[next_index]):
                issues.append(
                    f"line {start_line + index + 1}: native command is not followed "
                    "immediately by a $LASTEXITCODE guard"
                )
    return issues


class PowerShellWorkflowFailureTests(unittest.TestCase):
    def test_native_commands_cannot_be_masked_by_later_success(self) -> None:
        checked = 0
        failures: list[str] = []
        for path in sorted(WORKFLOWS.glob("*.yml")):
            text = path.read_text(encoding="utf-8")
            issues = native_failure_guard_issues(text)
            checked += sum(
                NATIVE_INVOCATION.search(line) is not None
                for _, block in literal_run_blocks(text)
                for line in block
            )
            failures.extend(f"{path.relative_to(ROOT)}: {issue}" for issue in issues)
        self.assertGreater(checked, 0, "lint did not inspect any native commands")
        self.assertEqual(failures, [])

    def test_lint_rejects_a_masked_failure(self) -> None:
        unsafe = """\
steps:
  - shell: pwsh
    run: |
      cargo test --locked
      Write-Output "a later command would mask the failure"
"""
        self.assertEqual(len(native_failure_guard_issues(unsafe)), 1)


if __name__ == "__main__":
    unittest.main()
