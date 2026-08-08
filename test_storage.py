"""Tests for SQLite A/N/O persistence and optional auto index."""

from __future__ import annotations

import os
import tempfile
import unittest

from memory import DEFAULT_SQLITE_AUTO, AdmittedMemory, NodeMemory, ObservationMemory
from storage import (
    connect,
    get_admitted,
    get_node,
    get_observation,
    init_db,
    insert_admitted,
    insert_node,
    insert_observation,
)
from util.index_alloc import allocate


class TestSqliteStore(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.db_path = os.path.join(self._tmp.name, "test.db")
        self.conn = connect(self.db_path)
        init_db(self.conn)

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_roundtrip_admitted_explicit(self) -> None:
        a = insert_admitted(
            self.conn, data="hello", from_agent=7, admitted_index=1
        )
        self.assertEqual(a, AdmittedMemory(1, "hello", 7))
        self.assertEqual(get_admitted(self.conn, 1), a)

    def test_admitted_sqlite_auto_via_allocate(self) -> None:
        idx = allocate(DEFAULT_SQLITE_AUTO, "admitted")
        self.assertIsNone(idx)
        a = insert_admitted(
            self.conn, data="auto", from_agent=1, admitted_index=idx
        )
        self.assertIsInstance(a.admitted_index, int)
        self.assertGreaterEqual(a.admitted_index, 1)
        self.assertEqual(get_admitted(self.conn, a.admitted_index), a)

    def test_roundtrip_node_and_branch(self) -> None:
        a1 = insert_admitted(self.conn, data="a", from_agent=1, admitted_index=1)
        a2 = insert_admitted(self.conn, data="b", from_agent=1, admitted_index=2)
        root = insert_node(
            self.conn,
            sandbox_id="sb",
            timestamp=1.0,
            parent_node_index=None,
            admitted_index=a1.admitted_index,
            node_index=10,
        )
        child = insert_node(
            self.conn,
            sandbox_id="sb",
            timestamp=2.0,
            parent_node_index=10,
            admitted_index=a2.admitted_index,
            node_index=None,  # auto
        )
        self.assertEqual(root, get_node(self.conn, 10))
        self.assertEqual(child.parent_node_index, 10)
        self.assertEqual(get_node(self.conn, child.node_index), child)

    def test_roundtrip_observation_tags_and_reobs(self) -> None:
        a = insert_admitted(self.conn, data="x", from_agent=1)
        n = insert_node(
            self.conn,
            sandbox_id="sb",
            timestamp=1.0,
            parent_node_index=None,
            admitted_index=a.admitted_index,
        )
        o1 = insert_observation(
            self.conn,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=n.node_index,
            source_observation_index=None,
            tags=frozenset({"str", "code"}),
            timestamp=3.0,
            observation_index=allocate(DEFAULT_SQLITE_AUTO, "observation"),
        )
        o2 = insert_observation(
            self.conn,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=n.node_index,
            source_observation_index=o1.observation_index,
            tags=frozenset({"summary"}),
            timestamp=4.0,
        )
        self.assertEqual(get_observation(self.conn, o1.observation_index), o1)
        self.assertEqual(get_observation(self.conn, o2.observation_index), o2)
        self.assertEqual(o2.source_observation_index, o1.observation_index)

    def test_fk_rejects_node_without_admitted(self) -> None:
        with self.assertRaises(Exception):
            insert_node(
                self.conn,
                sandbox_id="sb",
                timestamp=1.0,
                parent_node_index=None,
                admitted_index=999,
            )


if __name__ == "__main__":
    unittest.main()
