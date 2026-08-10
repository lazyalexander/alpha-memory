"""Tests for Memory Engine build_* (persist-then-complete)."""

from __future__ import annotations

import importlib.util
import os
import tempfile
import threading
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
        self.db_path = os.path.join(self._tmp.name, "e.db")
        self.conn = connect(self.db_path)
        init_db(self.conn)
        self.acq_a = partial(allocate, DEFAULT_SQLITE_AUTO, "admitted")
        self.acq_n = partial(allocate, DEFAULT_SQLITE_AUTO, "node")
        self.acq_o = partial(allocate, DEFAULT_SQLITE_AUTO, "observation")
        self.admit_lock = threading.Lock()

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_build_admitted_complete_after_insert(self) -> None:
        a = engine.build_admitted(
            self.conn,
            data="hello",
            from_agent=3,
            acquire_index=self.acq_a,
            admit_lock=self.admit_lock,
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
            self.conn,
            data="payload",
            from_agent=1,
            acquire_index=self.acq_a,
            admit_lock=self.admit_lock,
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
            admit_lock=self.admit_lock,
        )
        self.assertEqual(a.admitted_index, 42)

    def test_admit_lock_unique_and_ordered(self) -> None:
        """Concurrent admits with one lock: unique ids; ORDER BY index = order."""
        # Shared connection across threads; admit_lock serializes all access.
        self.conn.close()
        self.conn = connect(self.db_path, check_same_thread=False)

        n = 20
        results: list = []
        errors: list[BaseException] = []
        barrier = threading.Barrier(n)
        results_lock = threading.Lock()

        def worker(i: int) -> None:
            try:
                barrier.wait(timeout=5)
                row = engine.build_admitted(
                    self.conn,
                    data=f"d{i}",
                    from_agent=i,
                    acquire_index=self.acq_a,
                    admit_lock=self.admit_lock,
                )
                with results_lock:
                    results.append(row)
            except BaseException as e:
                with results_lock:
                    errors.append(e)

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(n)]
        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=10)

        self.assertEqual(errors, [])
        self.assertEqual(len(results), n)
        ids = [r.admitted_index for r in results]
        self.assertEqual(len(ids), len(set(ids)), "admitted_index must be unique")

        cur = self.conn.execute(
            "SELECT admitted_index FROM admitted ORDER BY admitted_index ASC"
        )
        ordered = [int(r[0]) for r in cur.fetchall()]
        self.assertEqual(ordered, sorted(ids))
        # sqlite_auto: indices are contiguous from first id over n admits
        self.assertEqual(ordered, list(range(ordered[0], ordered[0] + n)))


if __name__ == "__main__":
    unittest.main()
