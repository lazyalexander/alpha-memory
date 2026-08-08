"""Tests for MemoryPool: introduce, allocate, op log, multi-agent."""

from __future__ import annotations

import unittest

from memory import AdmittedMemory, MemoryPool, bootstrap_viewpoint


def _ok(_d) -> bool:
    return True


def _annotator(d) -> str:
    return "n"


def _vp_fn(_admitted: AdmittedMemory):
    return 0, frozenset({"v"}), 100.0


class TestPool(unittest.TestCase):
    def setUp(self) -> None:
        self.pool = MemoryPool("sandbox-1")
        self.vp = bootstrap_viewpoint(
            fn=_vp_fn,
            token=b"vp",
            digest=b"d1",
            index=0,
        )

    def test_introduce_provenance_and_op_log(self) -> None:
        agent = self.pool.register_agent()
        admitted = self.pool.introduce(
            "hello",
            self.vp,
            agent,
            via="remember",
            data_validator=_ok,
            annotator=_annotator,
        )
        self.assertIsNotNone(admitted)
        assert admitted is not None
        self.assertEqual(admitted.sandbox_id, "sandbox-1")
        self.assertEqual(admitted.introduced_by, agent)
        self.assertEqual(admitted.introduced_via, "remember")
        self.assertEqual(admitted.data_index, 1)
        ops = self.pool.ops_for_data(admitted.data_index)
        self.assertEqual(ops[0].name, "introduced")
        self.assertEqual(ops[0].agent_index, agent)

    def test_apply_view_registers_node(self) -> None:
        agent = self.pool.register_agent()
        admitted = self.pool.introduce(
            "payload",
            self.vp,
            agent,
            via="import",
            data_validator=_ok,
            annotator=_annotator,
        )
        assert admitted is not None
        node = self.pool.apply_view(admitted, agent_index=agent)
        self.assertIsNotNone(node)
        assert node is not None
        self.assertTrue(node.is_view_root)
        self.assertEqual(node.data_indexes, (admitted.data_index,))
        self.assertIs(self.pool.get_node(node.index), node)
        names = [e.name for e in self.pool.ops_for_node(node.index)]
        self.assertIn("view_applied", names)

    def test_allocate_uniqueness_and_two_agents(self) -> None:
        a1 = self.pool.register_agent()
        a2 = self.pool.register_agent()
        self.assertNotEqual(a1, a2)
        d1 = self.pool.allocate("data")
        d2 = self.pool.allocate("data")
        self.assertNotEqual(d1, d2)

    def test_merge_m1_via_pool(self) -> None:
        agent = self.pool.register_agent()
        ad1 = self.pool.introduce(
            "A", self.vp, agent, via="remember", data_validator=_ok, annotator=_annotator
        )
        ad2 = self.pool.introduce(
            "B", self.vp, agent, via="remember", data_validator=_ok, annotator=_annotator
        )
        assert ad1 is not None and ad2 is not None
        n1 = self.pool.apply_view(ad1, agent_index=agent)
        n2 = self.pool.apply_view(ad2, agent_index=agent)
        assert n1 is not None and n2 is not None
        merged = self.pool.reflect_merge(
            (n1, n2), self.vp, tags=frozenset({"m"}), agent_index=agent
        )
        self.assertIsNotNone(merged)
        assert merged is not None
        self.assertEqual(merged.op, "merge")
        self.assertEqual(set(merged.data_indexes), {ad1.data_index, ad2.data_index})
        self.assertEqual(len(merged.origins), 2)

    def test_empty_sandbox_id_rejected(self) -> None:
        with self.assertRaises(ValueError):
            MemoryPool("")


if __name__ == "__main__":
    unittest.main()
