"""
Append-only remember event log on local disk (Kafka-like segments).

- 1000 events per segment file: remember/seg_000001.log, ...
- One JSON object per line; fsync after each append
- Truncate incomplete trailing line on read
- event_id assigned by EventIdAllocator (not SQL)

Truth for domain state remains SQLite admitted rows after materialize.
This log is the basis for ordered crash replay.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Iterator, NamedTuple

EVENTS_PER_SEGMENT = 1000


class RememberEvent(NamedTuple):
    event_id: int
    data: str
    from_agent: int


def _segment_index(event_id: int) -> int:
    """1-based segment number for event_id >= 1."""
    return (event_id - 1) // EVENTS_PER_SEGMENT + 1


def _segment_path(log_dir: Path, segment_index: int) -> Path:
    return log_dir / f"seg_{segment_index:06d}.log"


def _atomic_write_int(path: Path, value: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(str(value))
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def _read_int(path: Path, default: int = 0) -> int:
    if not path.exists():
        return default
    text = path.read_text(encoding="utf-8").strip()
    if not text:
        return default
    return int(text)


class FileRememberLog:
    """
    Sandbox-local segment log under ``root/remember/``.

    Layout::
        root/
          event_id          # optional external allocator path (caller-owned)
          checkpoint        # last successfully materialized event_id
          remember/
            seg_000001.log
            ...
    """

    def __init__(
        self,
        root: str | Path,
        *,
        events_per_segment: int = EVENTS_PER_SEGMENT,
    ) -> None:
        self.root = Path(root)
        self.events_per_segment = events_per_segment
        self.log_dir = self.root / "remember"
        self.checkpoint_path = self.root / "checkpoint"
        self.log_dir.mkdir(parents=True, exist_ok=True)

    def append(self, event: RememberEvent) -> None:
        """Append one event; durable before return (fsync)."""
        if event.event_id < 1:
            raise ValueError("event_id must be >= 1")
        seg = (event.event_id - 1) // self.events_per_segment + 1
        path = _segment_path(self.log_dir, seg)
        line = (
            json.dumps(
                {
                    "event_id": event.event_id,
                    "data": event.data,
                    "from_agent": event.from_agent,
                },
                ensure_ascii=False,
                separators=(",", ":"),
            )
            + "\n"
        )
        with open(path, "a", encoding="utf-8") as f:
            f.write(line)
            f.flush()
            os.fsync(f.fileno())

    def load_checkpoint(self) -> int:
        """Last materialized event_id; 0 means none."""
        return _read_int(self.checkpoint_path, 0)

    def save_checkpoint(self, event_id: int) -> None:
        """Advance checkpoint (only forward)."""
        current = self.load_checkpoint()
        if event_id < current:
            raise ValueError(
                f"checkpoint cannot move backward: {event_id} < {current}"
            )
        _atomic_write_int(self.checkpoint_path, event_id)

    def iter_from(self, after_event_id: int) -> Iterator[RememberEvent]:
        """
        Yield events with event_id > after_event_id in ascending order.
        Skips corrupt trailing partial line in a segment.
        """
        start_id = after_event_id + 1
        if start_id < 1:
            start_id = 1
        start_seg = (start_id - 1) // self.events_per_segment + 1
        for path, num in self._segment_files():
            if num < start_seg:
                continue
            yield from self._read_segment(path, min_event_id=start_id)

    def max_event_id(self) -> int:
        """Largest event_id present in segment files; 0 if empty."""
        max_id = 0
        for path, _num in self._segment_files():
            for ev in self._read_segment(path, min_event_id=1):
                if ev.event_id > max_id:
                    max_id = ev.event_id
        return max_id

    def _segment_files(self) -> list[tuple[Path, int]]:
        out: list[tuple[Path, int]] = []
        for path in sorted(self.log_dir.glob("seg_*.log")):
            try:
                num = int(path.stem.split("_", 1)[1])
            except (IndexError, ValueError):
                continue
            out.append((path, num))
        return out

    def _read_segment(
        self, path: Path, *, min_event_id: int
    ) -> Iterator[RememberEvent]:
        if not path.exists():
            return
        with open(path, "rb") as f:
            raw = f.read()
        # Truncate incomplete last line (no trailing newline)
        if raw and not raw.endswith(b"\n"):
            last_nl = raw.rfind(b"\n")
            raw = raw[: last_nl + 1] if last_nl >= 0 else b""
        text = raw.decode("utf-8")
        for line in text.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
                ev = RememberEvent(
                    event_id=int(obj["event_id"]),
                    data=str(obj["data"]),
                    from_agent=int(obj["from_agent"]),
                )
            except (KeyError, TypeError, ValueError, json.JSONDecodeError):
                continue
            if ev.event_id >= min_event_id:
                yield ev
