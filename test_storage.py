"""Tests for SQLite A/N/O persistence."""

from __future__ import annotations

import os
import tempfile
import unittest

from memory import AdmittedMemory, NodeMemory, ObservationMemory
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


class TestSqliteStore(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.db_path = os.path.join(self._tmp.name, "test.db")
        self.conn = connect(self.db_path)
        init_db(self.conn)

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_roundtrip_admitted(self) -> None:
        a = AdmittedMemory(admitted_index=1, data="hello", from_agent=7)
        insert_admitted(self.conn, a)
        got = get_admitted(self.conn, 1)
        self.assertEqual(got, a)

    def test_roundtrip_node_and_branch(self) -> None:
        insert_admitted(
            self.conn, AdmittedMemory(admitted_index=1, data="a", from_agent=1)
        )
        insert_admitted(
            self.conn, AdmittedMemory(admitted_index=2, data="b", from_agent=1)
        )
        root = NodeMemory(
            node_index=10,
            sandbox_id="sb",
            timestamp=1.0,
            parent_node_index=None,
            admitted_index=1,
        )
        child = NodeMemory(
            node_index=11,
            sandbox_id="sb",
            timestamp=2.0,
            parent_node_index=10,
            admitted_index=2,
        )
        insert_node(self.conn, root)
        insert_node(self.conn, child)
        self.assertEqual(get_node(self.conn, 10), root)
        self.assertEqual(get_node(self.conn, 11), child)

    def test_roundtrip_observation_tags_and_reobs(self) -> None:
        insert_admitted(
            self.conn, AdmittedMemory(admitted_index=1, data="x", from_agent=1)
        )
        insert_node(
            self.conn,
            NodeMemory(
                node_index=10,
                sandbox_id="sb",
                timestamp=1.0,
                parent_node_index=None,
                admitted_index=1,
            ),
        )
        o1 = ObservationMemory(
            observation_index=100,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=10,
            source_observation_index=None,
            tags=frozenset({"str", "code"}),
            timestamp=3.0,
        )
        o2 = ObservationMemory(
            observation_index=101,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=10,
            source_observation_index=100,
            tags=frozenset({"summary"}),
            timestamp=4.0,
        )
        insert_observation(self.conn, o1)
        insert_observation(self.conn, o2)
        self.assertEqual(get_observation(self.conn, 100), o1)
        self.assertEqual(get_observation(self.conn, 101), o2)

    def test_fk_rejects_node_without_admitted(self) -> None:
        bad = NodeMemory(
            node_index=1,
            sandbox_id="sb",
            timestamp=1.0,
            parent_node_index=None,
            admitted_index=999,
        )
        with self.assertRaises(Exception):
            insert_node(self.conn, bad)


if __name__ == "__main__":
    unittest.main()
