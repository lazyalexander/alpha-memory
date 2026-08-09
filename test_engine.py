"""Tests for Memory Engine build_* (persist-then-complete)."""

from __future__ import annotations

import importlib.util
import os
import tempfile
import unittest
from functools import partial
from pathlib import Path

from memory import DEFAULT_SQLITE_AUTO
from storage import connect, get_admitted, get_node, get_observation, init_db
from util.index_alloc import allocate


def _load_engine():
    path = (
        Path(__file__).resolve().parent
        / "sandbox"
        / "memory engine"
        / "engine.py"
    )
    spec = importlib.util.spec_from_file_location("alpha_memory_engine", path)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


engine = _load_engine()


class TestEngineBuild(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.conn = connect(os.path.join(self._tmp.name, "e.db"))
        init_db(self.conn)
        self.acq_a = partial(allocate, DEFAULT_SQLITE_AUTO, "admitted")
        self.acq_n = partial(allocate, DEFAULT_SQLITE_AUTO, "node")
        self.acq_o = partial(allocate, DEFAULT_SQLITE_AUTO, "observation")

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_build_admitted_complete_after_insert(self) -> None:
        a = engine.build_admitted(
            self.conn,
            data="hello",
            from_agent=3,
            acquire_index=self.acq_a,
        )
        self.assertIsInstance(a.admitted_index, int)
        self.assertEqual(get_admitted(self.conn, a.admitted_index), a)

    def test_build_node_requires_admitted(self) -> None:
        with self.assertRaises(ValueError):
            engine.build_node(
                self.conn,
                sandbox_id="sb",
                timestamp=1.0,
                parent_node_index=None,
                admitted_index=99,
                acquire_index=self.acq_n,
            )

    def test_build_chain_a_n_o(self) -> None:
        a = engine.build_admitted(
            self.conn, data="payload", from_agent=1, acquire_index=self.acq_a
        )
        n = engine.build_node(
            self.conn,
            sandbox_id="sb-1",
            timestamp=10.0,
            parent_node_index=None,
            admitted_index=a.admitted_index,
            acquire_index=self.acq_n,
        )
        self.assertEqual(get_node(self.conn, n.node_index), n)
        o1 = engine.build_observation(
            self.conn,
            agent_id=1,
            viewpoint_index=2,
            source_node_index=n.node_index,
            source_observation_index=None,
            tags=frozenset({"t"}),
            timestamp=11.0,
            acquire_index=self.acq_o,
        )
        o2 = engine.build_observation(
            self.conn,
            agent_id=1,
            viewpoint_index=2,
            source_node_index=n.node_index,
            source_observation_index=o1.observation_index,
            tags=frozenset({"re"}),
            timestamp=12.0,
            acquire_index=self.acq_o,
        )
        self.assertEqual(get_observation(self.conn, o2.observation_index), o2)
        self.assertEqual(o2.source_observation_index, o1.observation_index)

    def test_explicit_index_fn(self) -> None:
        a = engine.build_admitted(
            self.conn,
            data="x",
            from_agent=0,
            acquire_index=lambda: 42,
        )
        self.assertEqual(a.admitted_index, 42)


if __name__ == "__main__":
    unittest.main()
