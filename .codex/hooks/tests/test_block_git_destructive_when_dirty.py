import importlib.util
import json
import pathlib
import sys
import tempfile
import unittest


def load_hook_module():
    hook_path = pathlib.Path(__file__).resolve().parents[1] / "block_git_destructive_when_dirty.py"
    spec = importlib.util.spec_from_file_location("block_git_destructive_when_dirty", hook_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


HOOK = load_hook_module()


class BlockGitDestructiveHookTests(unittest.TestCase):
    def test_git_clean_force_with_exclude_is_blocked(self):
        cmd = "git clean -f --exclude=node_modules"
        self.assertTrue(HOOK._git_clean_force_without_dry_run(cmd))

    def test_git_clean_dry_run_is_allowed(self):
        cmd = "git clean -fd -n"
        self.assertFalse(HOOK._git_clean_force_without_dry_run(cmd))

    def test_extract_command_context_prefers_exec_workdir(self):
        payload = {
            "tool_input": {
                "type": "function",
                "arguments": json.dumps(
                    {"cmd": "git reset --hard", "workdir": "/tmp/sandbox"}
                ),
            }
        }
        cmd, workdir = HOOK._extract_command_context(payload)
        self.assertEqual(cmd, "git reset --hard")
        self.assertEqual(workdir, "/tmp/sandbox")

    def test_git_switch_is_not_treated_as_destructive(self):
        cmd = "git switch main"
        matches = any(HOOK.re.search(pattern, cmd) for pattern in HOOK.DESTRUCTIVE_GIT_PATTERNS)
        self.assertFalse(matches)

    def test_non_git_workdir_is_unknown_state(self):
        with tempfile.TemporaryDirectory() as workdir:
            self.assertIsNone(HOOK._is_git_dirty(workdir))


if __name__ == "__main__":
    unittest.main()
