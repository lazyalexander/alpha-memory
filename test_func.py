"""Tests for func.py contracts and shape runners."""

from __future__ import annotations

import unittest

from func import (
    MEMORY_TOOLS,
    IndexRequest,
    ViewResult,
    ensure_tags,
    run_viewpoint,
    viewpoint_identity,
)


class TestFunc(unittest.TestCase):
    def test_no_update_in_memory_tools(self) -> None:
        self.assertNotIn("update", MEMORY_TOOLS)
        self.assertIn("remember", MEMORY_TOOLS)
        self.assertIn("forget", MEMORY_TOOLS)

    def test_run_viewpoint_legacy_triple(self) -> None:
        def fn(_admitted):
            return 7, frozenset({"a"}), 1.5

        vr = run_viewpoint(fn, object())
        self.assertIsNotNone(vr)
        assert vr is not None
        self.assertEqual(vr.suggested_node_index, 7)
        self.assertEqual(vr.tags, frozenset({"a"}))
        self.assertEqual(vr.timestamp, 1.5)
        self.assertEqual(vr.index_request.kind, "view")

    def test_run_viewpoint_view_result(self) -> None:
        def fn(_admitted):
            return ViewResult(
                tags=frozenset({"x"}),
                timestamp=9,
                index_request=IndexRequest(kind="view"),
                suggested_node_index=3,
            )

        vr = run_viewpoint(fn, object())
        assert vr is not None
        self.assertEqual(vr.suggested_node_index, 3)

    def test_run_viewpoint_bad_shape(self) -> None:
        self.assertIsNone(run_viewpoint(lambda _a: "nope", object()))
        self.assertIsNone(run_viewpoint(lambda _a: None, object()))

        def boom(_a):
            raise RuntimeError("fail")

        self.assertIsNone(run_viewpoint(boom, object()))

    def test_callable_object_protocol(self) -> None:
        class VP:
            def __call__(self, _admitted):
                return 1, frozenset(), 0.0

        vr = run_viewpoint(VP(), object())
        self.assertIsNotNone(vr)

    def test_ensure_tags_and_identity(self) -> None:
        self.assertEqual(ensure_tags(["a", "a"]), frozenset({"a"}))
        self.assertEqual(viewpoint_identity(b"t", b"d"), (b"t", b"d"))


if __name__ == "__main__":
    unittest.main()
