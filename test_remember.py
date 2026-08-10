"""Tests for file segment remember log + materialize to admitted."""

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
from storage.remember_log import EVENTS_PER_SEGMENT, FileRememberLog, RememberEvent
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


class TestFileRememberLog(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = self._tmp.name
        self.log = FileRememberLog(self.root)
        self.alloc = FileEventIdAllocator(os.path.join(self.root, "event_id"))

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_segment_rotation_1000(self) -> None:
        # event 1000 -> seg 1; event 1001 -> seg 2
        self.assertEqual(EVENTS_PER_SEGMENT, 1000)
        self.log.append(RememberEvent(1000, "a", 1))
        self.log.append(RememberEvent(1001, "b", 1))
        segs = sorted((Path(self.root) / "remember").glob("seg_*.log"))
        self.assertEqual([p.name for p in segs], ["seg_000001.log", "seg_000002.log"])

    def test_iter_from_checkpoint(self) -> None:
        for i in range(1, 4):
            self.log.append(RememberEvent(i, f"d{i}", 1))
        self.log.save_checkpoint(1)
        ids = [e.event_id for e in self.log.iter_from(self.log.load_checkpoint())]
        self.assertEqual(ids, [2, 3])


class TestRememberPipeline(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        base = self._tmp.name
        self.conn = connect(os.path.join(base, "r.db"), check_same_thread=False)
        init_db(self.conn)
        self.log = FileRememberLog(base)
        self.allocator = FileEventIdAllocator(os.path.join(base, "event_id"))
        self.admit_lock = threading.Lock()
        self.id_lock = threading.Lock()

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_validate_stub(self) -> None:
        self.assertTrue(admit_validate("x", 1))

    def test_submit_only_no_admitted_until_consumer(self) -> None:
        ticket = submit_remember_event(
            self.log,
            self.allocator,
            data="only-log",
            from_agent=1,
            id_lock=self.id_lock,
        )
        assert ticket is not None
        self.assertIsNone(get_admitted(self.conn, ticket.event_id))
        self.assertEqual(self.log.load_checkpoint(), 0)

    def test_remember_truth_is_admitted_row(self) -> None:
        row = remember(
            self.conn,
            self.log,
            self.allocator,
            data="hello",
            from_agent=2,
            admit_lock=self.admit_lock,
            id_lock=self.id_lock,
        )
        assert row is not None
        self.assertEqual(row.admitted_index, 1)
        self.assertEqual(row.data, "hello")
        self.assertEqual(get_admitted(self.conn, 1), row)
        self.assertEqual(self.log.load_checkpoint(), 1)

    def test_replay_skips_existing_admitted(self) -> None:
        t = submit_remember_event(
            self.log, self.allocator, data="x", from_agent=1, id_lock=self.id_lock
        )
        assert t is not None
        consumer.process_pending_once(
            self.conn, self.log, admit_lock=self.admit_lock
        )
        # second pass: checkpoint already at end
        again = consumer.process_pending_once(
            self.conn, self.log, admit_lock=self.admit_lock
        )
        self.assertEqual(again, [])
        self.assertIsNotNone(get_admitted(self.conn, t.event_id))

    def test_crash_replay_from_log(self) -> None:
        """Log has events, checkpoint behind, consumer catches up."""
        submit_remember_event(
            self.log, self.allocator, data="a", from_agent=0, id_lock=self.id_lock
        )
        submit_remember_event(
            self.log, self.allocator, data="b", from_agent=0, id_lock=self.id_lock
        )
        # no materialize yet
        self.assertIsNone(get_admitted(self.conn, 1))
        built = consumer.process_pending_once(
            self.conn, self.log, admit_lock=self.admit_lock
        )
        self.assertEqual([b.data for b in built], ["a", "b"])
        self.assertEqual(self.log.load_checkpoint(), 2)

    def test_remember_async(self) -> None:
        async def run() -> None:
            row = await remember_async(
                self.conn,
                self.log,
                self.allocator,
                data="async-payload",
                from_agent=9,
                admit_lock=self.admit_lock,
                id_lock=self.id_lock,
            )
            self.assertIsNotNone(row)
            assert row is not None
            self.assertEqual(get_admitted(self.conn, row.admitted_index), row)

        asyncio.run(run())


if __name__ == "__main__":
    unittest.main()
