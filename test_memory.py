"""Smoke tests for A/N/O memory records (data only)."""

from __future__ import annotations

import unittest

from memory import AdmittedMemory, NodeMemory, ObservationMemory


class TestAdmittedMemory(unittest.TestCase):
    def test_fields(self) -> None:
        a = AdmittedMemory(admitted_index=1, data="hello", from_agent=7)
        self.assertEqual(a.admitted_index, 1)
        self.assertEqual(a.data, "hello")
        self.assertEqual(a.from_agent, 7)

    def test_immutable(self) -> None:
        a = AdmittedMemory(admitted_index=1, data="x", from_agent=0)
        with self.assertRaises(AttributeError):
            a.data = "y"  # type: ignore[misc]


class TestNodeMemory(unittest.TestCase):
    def test_pointer_only_no_payload(self) -> None:
        n = NodeMemory(
            node_index=10,
            sandbox_id="sb-1",
            timestamp=100.0,
            parent_node_index=None,
            admitted_index=1,
        )
        self.assertEqual(n.admitted_index, 1)
        self.assertIsNone(n.parent_node_index)
        self.assertFalse(hasattr(n, "data"))

    def test_branch_parent(self) -> None:
        child = NodeMemory(
            node_index=11,
            sandbox_id="sb-1",
            timestamp=200.0,
            parent_node_index=10,
            admitted_index=2,
        )
        self.assertEqual(child.parent_node_index, 10)
        self.assertEqual(child.admitted_index, 2)


class TestObservationMemory(unittest.TestCase):
    def test_over_node(self) -> None:
        o = ObservationMemory(
            observation_index=100,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=10,
            source_observation_index=None,
            tags=frozenset({"code", "str"}),
            timestamp=300.0,
        )
        self.assertIsNone(o.source_observation_index)
        self.assertEqual(o.source_node_index, 10)
        self.assertEqual(o.tags, frozenset({"code", "str"}))
        self.assertFalse(hasattr(o, "reading"))
        self.assertFalse(hasattr(o, "importance"))

    def test_reobservation_link(self) -> None:
        o2 = ObservationMemory(
            observation_index=101,
            agent_id=7,
            viewpoint_index=3,
            source_node_index=10,
            source_observation_index=100,
            tags=frozenset({"summary"}),
            timestamp=301.0,
        )
        self.assertEqual(o2.source_observation_index, 100)


if __name__ == "__main__":
    unittest.main()
