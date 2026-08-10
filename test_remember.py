"""Tests for remember ingress tool."""

from __future__ import annotations

import os
import tempfile
import threading
import unittest
from functools import partial
from queue import Queue

from memory import DEFAULT_SQLITE_AUTO
from memory.tools.remember import (
    RememberRequest,
    admit_validate,
    process_remember_queue,
    remember,
    submit_remember,
)
from storage import connect, get_admitted, init_db
from util.index_alloc import allocate


class TestRemember(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.conn = connect(
            os.path.join(self._tmp.name, "r.db"),
            check_same_thread=False,
        )
        init_db(self.conn)
        self.acq = partial(allocate, DEFAULT_SQLITE_AUTO, "admitted")
        self.lock = threading.Lock()

    def tearDown(self) -> None:
        self.conn.close()
        self._tmp.cleanup()

    def test_admit_validate_stub(self) -> None:
        self.assertTrue(admit_validate("x", 1))
        self.assertTrue(admit_validate("", 0))

    def test_remember_sync(self) -> None:
        row = remember(
            self.conn,
            data="hello",
            from_agent=2,
            acquire_index=self.acq,
            admit_lock=self.lock,
        )
        self.assertIsNotNone(row)
        assert row is not None
        self.assertEqual(get_admitted(self.conn, row.admitted_index), row)

    def test_queue_bounds_writes(self) -> None:
        q: Queue[RememberRequest] = Queue()
        submit_remember(q, RememberRequest(data="a", from_agent=1))
        submit_remember(q, RememberRequest(data="b", from_agent=1))
        submit_remember(q, RememberRequest(data="c", from_agent=1))
        # only process two
        built = process_remember_queue(
            q,
            self.conn,
            acquire_index=self.acq,
            admit_lock=self.lock,
            max_items=2,
        )
        self.assertEqual(len(built), 2)
        self.assertEqual(built[0].data, "a")
        self.assertEqual(built[1].data, "b")
        # one left on queue
        rest = process_remember_queue(
            q,
            self.conn,
            acquire_index=self.acq,
            admit_lock=self.lock,
        )
        self.assertEqual(len(rest), 1)
        self.assertEqual(rest[0].data, "c")


if __name__ == "__main__":
    unittest.main()
