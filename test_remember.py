"""Tests for durable remember event + materialize to admitted."""

from __future__ import annotations

import asyncio
import importlib.util
import os
import tempfile
import threading
import unittest
from pathlib import Path

from memory.tools.remember import (
    admit_validate,
    remember,
    remember_async,
    submit_remember_event,
)
from storage import connect, get_admitted, init_db
from storage.remember_log import get_event, list_pending
from util.event_id import FileEventIdAllocator


def _load_consumer():
    path = (
        Path(__file__).resolve().parent
        / "sandbox"
        / "memory engine"
        / "admit_consumer.py"
    )
    spec = importlib.util.spec_from_file_location("admit_consumer_test", path)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


consumer = _load_consumer()


class TestRememberPipeline(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        base = self._tmp.name
        self.conn = connect(os.path.join(base, "r.db"), check_same_thread=False)
        init_db(self.conn)
        self.allocator = FileEventIdAllocator(os.path.join(base, "event_id"))
        self.admit_lock = threading.Lock()
        self.id_lock = threading.Lock()

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_validate_stub(self) -> None:
        self.assertTrue(admit_validate("x", 1))

    def test_submit_only_pending_no_admitted(self) -> None:
        ticket = submit_remember_event(
            self.conn,
            self.allocator,
            data="only-event",
            from_agent=1,
            id_lock=self.id_lock,
        )
        assert ticket is not None
        self.assertIsNone(get_admitted(self.conn, ticket.event_id))
        ev = get_event(self.conn, ticket.event_id)
        assert ev is not None
        self.assertEqual(ev.status, "pending")
        self.assertEqual(list_pending(self.conn)[0].event_id, ticket.event_id)

    def test_remember_requires_admitted_row(self) -> None:
        row = remember(
            self.conn,
            self.allocator,
            data="hello",
            from_agent=2,
            admit_lock=self.admit_lock,
            id_lock=self.id_lock,
        )
        assert row is not None
        self.assertEqual(row.data, "hello")
        # admitted_index := event_id
        self.assertEqual(row.admitted_index, 1)
        self.assertEqual(get_admitted(self.conn, 1), row)
        ev = get_event(self.conn, 1)
        assert ev is not None
        self.assertEqual(ev.status, "done")

    def test_event_id_monotonic_independent_of_sql_auto(self) -> None:
        t1 = submit_remember_event(
            self.conn, self.allocator, data="a", from_agent=0, id_lock=self.id_lock
        )
        t2 = submit_remember_event(
            self.conn, self.allocator, data="b", from_agent=0, id_lock=self.id_lock
        )
        assert t1 and t2
        self.assertEqual(t1.event_id, 1)
        self.assertEqual(t2.event_id, 2)
        consumer.process_pending_once(self.conn, admit_lock=self.admit_lock)
        self.assertEqual(get_admitted(self.conn, 1).data, "a")  # type: ignore[union-attr]
        self.assertEqual(get_admitted(self.conn, 2).data, "b")  # type: ignore[union-attr]

    def test_idempotent_rematerialize(self) -> None:
        ticket = submit_remember_event(
            self.conn, self.allocator, data="x", from_agent=1, id_lock=self.id_lock
        )
        assert ticket is not None
        consumer.process_pending_once(self.conn, admit_lock=self.admit_lock)
        # pending empty; second pass no-op
        again = consumer.process_pending_once(self.conn, admit_lock=self.admit_lock)
        self.assertEqual(again, [])
        self.assertIsNotNone(get_admitted(self.conn, ticket.event_id))

    def test_remember_async(self) -> None:
        async def run() -> None:
            row = await remember_async(
                self.conn,
                self.allocator,
                data="async-payload",
                from_agent=9,
                admit_lock=self.admit_lock,
                id_lock=self.id_lock,
            )
            self.assertIsNotNone(row)
            assert row is not None
            self.assertEqual(row.admitted_index, row.admitted_index)
            self.assertEqual(get_admitted(self.conn, row.admitted_index), row)

        asyncio.run(run())


if __name__ == "__main__":
    unittest.main()
