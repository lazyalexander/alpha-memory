"""Tests for pluggable index allocation."""

from __future__ import annotations

import unittest

from memory import DEFAULT_SQLITE_AUTO, IndexStrategyConfig
from util.index_alloc import allocate, register_strategy, strategy_sqlite_auto


class TestIndexAlloc(unittest.TestCase):
    def test_sqlite_auto_returns_none(self) -> None:
        self.assertIsNone(strategy_sqlite_auto())
        for kind in ("admitted", "node", "observation"):
            self.assertIsNone(allocate(DEFAULT_SQLITE_AUTO, kind))

    def test_per_kind_strategy_from_config(self) -> None:
        register_strategy("fixed_7", lambda: 7)
        cfg = IndexStrategyConfig(
            admitted="sqlite_auto",
            node="fixed_7",
            observation="sqlite_auto",
        )
        self.assertIsNone(allocate(cfg, "admitted"))
        self.assertEqual(allocate(cfg, "node"), 7)
        self.assertIsNone(allocate(cfg, "observation"))

    def test_unknown_strategy(self) -> None:
        cfg = IndexStrategyConfig(
            admitted="nope",
            node="sqlite_auto",
            observation="sqlite_auto",
        )
        with self.assertRaises(KeyError):
            allocate(cfg, "admitted")


if __name__ == "__main__":
    unittest.main()
