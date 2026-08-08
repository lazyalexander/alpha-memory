"""Tests for memory foundation: viewpoint-as-memory + admit."""

from __future__ import annotations

import unittest

from memory import (
    AdmittedMemory,
    abstract_memory,
    admit_memory,
    apply_viewpoint,
    bootstrap_viewpoint,
    is_stale,
    reabstract,
    reflect_forget,
    reflect_merge,
    root_of,
    same_source,
    viewpoint_matches,
    walk_origin,
    walk_origins,
)


def _always_ok(_data) -> bool:
    return True


def _index_ok(_i: int) -> bool:
    return True


def _annotator(data) -> str:
    return f"note:{data!r}"


def _viewpoint_fn_factory(index: int, tags: frozenset[str], ts: float):
    def _fn(admitted: AdmittedMemory):
        return index, tags, ts

    return _fn


class TestMemorySubstrate(unittest.TestCase):
    def test_bootstrap_admit_apply_root(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(10, frozenset({"v1"}), 1000.0),
            token=b"vp-a",
            digest=b"digest-1",
            index=0,
            tags=frozenset({"genesis"}),
            timestamp=1,
            meta="genesis viewpoint",
        )
        payload = {"text": "hello"}
        admitted = admit_memory(
            payload,
            vp,
            _always_ok,
            _annotator,
            index=1,
            index_validator=_index_ok,
        )
        self.assertIsNotNone(admitted)
        assert admitted is not None
        self.assertIs(admitted.data, payload)
        self.assertIs(admitted.viewpoint, vp)

        root = apply_viewpoint(admitted)
        self.assertIsNotNone(root)
        assert root is not None
        self.assertIsNone(root.origin)
        self.assertTrue(root.is_view_root)
        self.assertEqual(root.timestamp, 1000.0)
        self.assertEqual(root.index, 10)
        self.assertEqual(root.tags, frozenset({"v1"}))
        self.assertIs(root.source, admitted)
        self.assertIs(root.source.data, payload)

    def test_abstract_chain_shares_source_no_timestamp(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset({"base"}), 50.0),
            token=b"vp-a",
            digest=b"d1",
            index=0,
        )
        data = "raw-mem"
        admitted = admit_memory(data, vp, _always_ok, _annotator, 0, _index_ok)
        assert admitted is not None
        root = apply_viewpoint(admitted)
        assert root is not None

        abs_vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(99, frozenset(), 0.0),
            token=b"abs-1",
            digest=b"abs-d",
            index=2,
        )

        def declare(info, tags, viewpoint):
            if viewpoint.mark.token != b"vp-a":
                return None

            def fc(original):
                return f"abstracted:{original}"

            return fc, abs_vp

        child = abstract_memory(root, declare, index=20, tags=frozenset({"L2"}))
        self.assertIsNotNone(child)
        assert child is not None
        self.assertIs(child.origin, root)
        self.assertIsNone(child.timestamp)
        self.assertIs(child.source, root.source)
        self.assertIs(child.source.data, data)
        self.assertEqual(child.viewpoint.mark.token, b"abs-1")

        chain = list(walk_origin(child))
        self.assertEqual(len(chain), 2)
        self.assertIs(root_of(child), root)

    def test_multi_branch_same_data_different_viewpoint(self) -> None:
        data = ["item"]
        vp1 = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset({"a"}), 10.0),
            token=b"t1",
            digest=b"d1",
            index=0,
        )
        vp2 = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(2, frozenset({"b"}), 20.0),
            token=b"t1",
            digest=b"d2",
            index=0,
        )
        a1 = AdmittedMemory(data, vp1, 0, _annotator, _index_ok)
        a2 = AdmittedMemory(data, vp2, 0, _annotator, _index_ok)
        r1 = apply_viewpoint(a1)
        r2 = apply_viewpoint(a2)
        assert r1 is not None and r2 is not None
        self.assertIs(r1.source.data, r2.source.data)
        self.assertIs(r1.source.data, data)
        self.assertFalse(viewpoint_matches(r1.viewpoint, r2.viewpoint))
        self.assertEqual(r1.tags, frozenset({"a"}))
        self.assertEqual(r2.tags, frozenset({"b"}))

    def test_stale_on_digest_change(self) -> None:
        vp_old = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"same",
            digest=b"old",
            index=0,
        )
        admitted = admit_memory("x", vp_old, _always_ok, _annotator, 0, _index_ok)
        assert admitted is not None
        root = apply_viewpoint(admitted)
        assert root is not None

        vp_new = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"same",
            digest=b"new",
            index=0,
        )
        self.assertTrue(is_stale(root, vp_new))
        self.assertFalse(is_stale(root, vp_old))

    def test_empty_data_rejected(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"t",
            digest=b"d",
            index=0,
        )
        self.assertIsNone(admit_memory("", vp, _always_ok, _annotator, 0, _index_ok))
        self.assertIsNone(admit_memory([], vp, _always_ok, _annotator, 0, _index_ok))
        self.assertIsNone(admit_memory(None, vp, _always_ok, _annotator, 0, _index_ok))

    def test_declare_refuse(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"vp",
            digest=b"d",
            index=0,
        )
        admitted = admit_memory("ok", vp, _always_ok, _annotator, 0, _index_ok)
        assert admitted is not None
        root = apply_viewpoint(admitted)
        assert root is not None

        def refuse(_info, _tags, _vp):
            return None

        self.assertIsNone(abstract_memory(root, refuse, index=1))

    def test_reabstract_new_branch(self) -> None:
        data = "original"
        vp1 = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset({"old"}), 1.0),
            token=b"t",
            digest=b"1",
            index=0,
        )
        vp2 = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(2, frozenset({"new"}), 2.0),
            token=b"t",
            digest=b"2",
            index=0,
        )
        b1 = reabstract(data, vp1, _always_ok, _annotator, 0, _index_ok)
        b2 = reabstract(data, vp2, _always_ok, _annotator, 0, _index_ok)
        assert b1 is not None and b2 is not None
        self.assertEqual(b1.source.data, b2.source.data)
        self.assertIsNot(b1, b2)
        self.assertIsNone(b1.origin)
        self.assertIsNone(b2.origin)

    def test_same_source_helper(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"t",
            digest=b"d",
            index=0,
        )
        admitted = admit_memory("s", vp, _always_ok, _annotator, 0, _index_ok)
        assert admitted is not None
        root = apply_viewpoint(admitted)
        assert root is not None

        abs_vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(0, frozenset(), 0.0),
            token=b"a",
            digest=b"a",
            index=1,
        )

        def declare(_i, _t, _v):
            return (lambda d: d), abs_vp

        child = abstract_memory(root, declare, index=3)
        assert child is not None
        self.assertTrue(same_source(root, child))

    def test_reflect_merge_m1(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset({"a"}), 1.0),
            token=b"t",
            digest=b"d",
            index=0,
        )
        a1 = admit_memory("one", vp, _always_ok, _annotator, 10, _index_ok)
        a2 = admit_memory("two", vp, _always_ok, _annotator, 20, _index_ok)
        assert a1 is not None and a2 is not None
        r1 = apply_viewpoint(a1)
        r2 = apply_viewpoint(a2)
        assert r1 is not None and r2 is not None
        merged = reflect_merge((r1, r2), index=99, viewpoint=vp, tags=frozenset({"m"}))
        self.assertIsNotNone(merged)
        assert merged is not None
        self.assertEqual(merged.op, "merge")
        self.assertIsNone(merged.origin)
        self.assertEqual(len(merged.origins), 2)
        self.assertEqual(set(merged.data_indexes), {10, 20})
        walked = list(walk_origins(merged))
        self.assertGreaterEqual(len(walked), 3)

    def test_reflect_forget_tombstone(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"t",
            digest=b"d",
            index=0,
        )
        admitted = admit_memory("x", vp, _always_ok, _annotator, 1, _index_ok)
        assert admitted is not None
        root = apply_viewpoint(admitted)
        assert root is not None
        tomb = reflect_forget(root, index=50, viewpoint=vp)
        assert tomb is not None
        self.assertEqual(tomb.op, "forget")
        self.assertIs(tomb.origin, root)
        self.assertIs(tomb.source.data, admitted.data)

    def test_introduction_fields_on_admit(self) -> None:
        vp = bootstrap_viewpoint(
            fn=_viewpoint_fn_factory(1, frozenset(), 1.0),
            token=b"t",
            digest=b"d",
            index=0,
        )
        admitted = admit_memory(
            "z",
            vp,
            _always_ok,
            _annotator,
            5,
            _index_ok,
            sandbox_id="sb",
            introduced_by=3,
            introduced_via="import",
        )
        assert admitted is not None
        self.assertEqual(admitted.sandbox_id, "sb")
        self.assertEqual(admitted.introduced_by, 3)
        self.assertEqual(admitted.introduced_via, "import")
        self.assertEqual(admitted.data_index, 5)


if __name__ == "__main__":
    unittest.main()
