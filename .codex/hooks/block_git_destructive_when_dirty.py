#!/usr/bin/env python3

import json
import re
import shlex
import subprocess
import sys
from typing import Any, Optional


DESTRUCTIVE_GIT_PATTERNS = [
    # Discards working tree changes
    r"\bgit\s+reset\s+--hard\b",
    r"\bgit\s+checkout\s+--\b",
    r"\bgit\s+restore\b",
]

GIT_CLEAN_FORCE_PATTERN = r"\bgit\s+clean\b"


def _extract_command_context(payload: dict[str, Any]) -> tuple[str, Optional[str]]:
    tool_input = payload.get("tool_input")
    if not isinstance(tool_input, dict):
        return ("", None)

    tool_type = tool_input.get("type")
    if tool_type == "function":
        args = tool_input.get("arguments")
        if not isinstance(args, str) or not args:
            return ("", None)
        try:
            obj = json.loads(args)
        except json.JSONDecodeError:
            return (args, None)

        # exec_command tool
        if isinstance(obj, dict) and isinstance(obj.get("cmd"), str):
            workdir = obj.get("workdir")
            return (obj["cmd"], workdir if isinstance(workdir, str) else None)

        # shell tool
        if isinstance(obj, dict) and isinstance(obj.get("command"), list):
            if all(isinstance(x, str) for x in obj["command"]):
                workdir = obj.get("cwd")
                return (" ".join(obj["command"]), workdir if isinstance(workdir, str) else None)

        # shell_command tool
        if isinstance(obj, dict) and isinstance(obj.get("command"), str):
            workdir = obj.get("cwd")
            return (obj["command"], workdir if isinstance(workdir, str) else None)

        return (args, None)

    if tool_type == "local_shell":
        cmd = tool_input.get("command")
        if isinstance(cmd, list) and all(isinstance(x, str) for x in cmd):
            workdir = tool_input.get("cwd")
            return (" ".join(cmd), workdir if isinstance(workdir, str) else None)

    return ("", None)


def _is_git_dirty(workdir: Optional[str]) -> Optional[bool]:
    try:
        # `--porcelain=v1` is stable and easy to parse.
        result = subprocess.run(
            ["git", "status", "--porcelain=v1"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            cwd=workdir,
        )
    except OSError:
        return None

    # If not a git repo, git returns non-zero; treat as "unknown".
    if result.returncode != 0:
        return None
    return result.stdout.strip() != ""


def _iter_command_tokens(cmd_text: str) -> list[str]:
    try:
        return shlex.split(cmd_text, posix=True)
    except ValueError:
        return cmd_text.split()


def _git_clean_without_dry_run_tokens(tokens: list[str]) -> bool:
    separators = {"&&", "||", ";", "|"}
    index = 0
    while index < len(tokens):
        if tokens[index] != "git":
            index += 1
            continue
        cursor = index + 1
        clean_index: Optional[int] = None
        while cursor < len(tokens) and tokens[cursor] not in separators:
            token = tokens[cursor]
            if token == "clean":
                clean_index = cursor
                break
            if token in {"-C", "--git-dir", "--work-tree"}:
                cursor += 2
                continue
            if token.startswith("-"):
                cursor += 1
                continue
            break
        if clean_index is None:
            index += 1
            continue

        has_force = False
        has_dry_run = False
        cursor = clean_index + 1
        while cursor < len(tokens) and tokens[cursor] not in separators:
            token = tokens[cursor]
            if token == "--force":
                has_force = True
            elif token == "--dry-run":
                has_dry_run = True
            elif token.startswith("--"):
                pass
            elif token.startswith("-") and token != "-":
                flags = token[1:]
                has_force = has_force or ("f" in flags)
                has_dry_run = has_dry_run or ("n" in flags)
            cursor += 1

        if has_force and not has_dry_run:
            return True

        index = cursor
    return False

def _git_clean_force_without_dry_run(cmd_text: str) -> bool:
    if not re.search(GIT_CLEAN_FORCE_PATTERN, cmd_text):
        return False

    # We only block "forceful" clean variants. Typical forms:
    # - git clean -f
    # - git clean -fd
    # - git clean -f -d
    # - git clean -ff
    # We allow dry-run:
    # - git clean -n -f
    # - git clean -f -n
    # - git clean --dry-run -f
    # - git clean -f --dry-run
    #
    tokens = _iter_command_tokens(cmd_text)
    return _git_clean_without_dry_run_tokens(tokens)


def main() -> int:
    payload = json.load(sys.stdin)
    cmd_text, workdir = _extract_command_context(payload)
    if not cmd_text:
        return 0

    # Policy: always block `git clean -f` unless it's a dry-run (`-n` / `--dry-run`).
    # This is intentionally independent of "dirty" state, because it deletes untracked files.
    if _git_clean_force_without_dry_run(cmd_text):
        print("BLOCKED: `git clean -f` is denied (use `git clean -n -f` to preview).", file=sys.stderr)
        print(f"Attempted: {cmd_text}", file=sys.stderr)
        return 2

    if not any(re.search(p, cmd_text) for p in DESTRUCTIVE_GIT_PATTERNS):
        return 0

    dirty = _is_git_dirty(workdir)
    if dirty is True or dirty is None:
        print(
            "BLOCKED: working tree has uncommitted changes (or state unknown).",
            file=sys.stderr,
        )
        print(f"Attempted: {cmd_text}", file=sys.stderr)
        print(
            "Commit/stash your changes, or run the command manually if you really intend it.",
            file=sys.stderr,
        )
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
