import datetime as dt
import importlib.util
import pathlib
import sys
import unittest


def load_module():
    script_path = pathlib.Path(__file__).resolve().parents[1] / "run_session_ledger_shards.py"
    spec = importlib.util.spec_from_file_location("run_session_ledger_shards", script_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


RUN_SHARDS = load_module()


class RunSessionLedgerShardsTests(unittest.TestCase):
    def test_split_into_shards_rejects_non_positive_workers(self):
        since = dt.date(2026, 1, 1)
        until = dt.date(2026, 1, 2)
        with self.assertRaisesRegex(ValueError, "workers must be >= 1"):
            RUN_SHARDS._split_into_shards(since, until, 0)

    def test_split_into_shards_allows_positive_workers(self):
        since = dt.date(2026, 1, 1)
        until = dt.date(2026, 1, 3)
        shards = RUN_SHARDS._split_into_shards(since, until, 2)
        self.assertEqual([s.shard_id for s in shards], ["2026-01-01..2026-01-01", "2026-01-02..2026-01-03"])


if __name__ == "__main__":
    unittest.main()
