"""
pool.py — MemoryPool (runtime registry; not a memory kind).

Sandbox-scoped: allocate indices, introduce originals, register abstract/reflect
nodes, append-only op logs. Does not implement Memory tools product API.
"""

from __future__ import annotations

from typing import Any, Callable, NamedTuple, Union

from func import IndexKind

from memory.abstract import AbstractMemory, abstract_memory, apply_viewpoint
from memory.common import (
    Annotator,
    DataValidator,
    ImmutableData,
    ImmutableIndex,
    IndexValidator,
    MemoryTags,
    SandboxId,
)
from memory.original import AdmittedMemory, ViewpointMemory, admit_memory
from memory.reflect import ReflectMemory, reflect_forget, reflect_merge

StructuralNode = Union[AbstractMemory, ReflectMemory]


class OpEvent(NamedTuple):
    seq: int
    name: str
    agent_index: ImmutableIndex | None
    detail: Any = None


class AgentRecord(NamedTuple):
    agent_index: ImmutableIndex


def _mem_key_data(data_index: ImmutableIndex) -> str:
    return f"data:{data_index}"


def _mem_key_node(node_index: ImmutableIndex) -> str:
    return f"node:{node_index}"


class MemoryPool:
    """Sandbox-scoped memory registry + index allocator + introduce/op-log gate."""

    def __init__(self, sandbox_id: SandboxId) -> None:
        if not sandbox_id:
            raise ValueError("sandbox_id must be non-empty")
        self.sandbox_id: SandboxId = sandbox_id
        self._seq: dict[IndexKind, int] = {
            "data": 0,
            "view": 0,
            "reflection": 0,
            "agent": 0,
            "agent_action": 0,
        }
        self._data: dict[ImmutableIndex, AdmittedMemory] = {}
        self._nodes: dict[ImmutableIndex, StructuralNode] = {}
        self._agents: dict[ImmutableIndex, AgentRecord] = {}
        self._ops: dict[str, list[OpEvent]] = {}
        self._op_seq: dict[str, int] = {}

    def allocate(self, kind: IndexKind) -> ImmutableIndex:
        self._seq[kind] = self._seq[kind] + 1
        return self._seq[kind]

    def register_agent(self) -> ImmutableIndex:
        agent_index = self.allocate("agent")
        self._agents[agent_index] = AgentRecord(agent_index=agent_index)
        self._append(f"agent:{agent_index}", "agent_registered", agent_index, None)
        return agent_index

    def introduce(
        self,
        data: ImmutableData,
        viewpoint: ViewpointMemory,
        agent_index: ImmutableIndex,
        via: str,
        data_validator: DataValidator,
        annotator: Annotator,
        index_validator: IndexValidator | None = None,
    ) -> AdmittedMemory | None:
        if agent_index not in self._agents:
            return None
        if not via:
            return None

        def _ok_index(_i: ImmutableIndex) -> bool:
            return True

        validator = index_validator if index_validator is not None else _ok_index
        data_index = self.allocate("data")
        admitted = admit_memory(
            data,
            viewpoint,
            data_validator,
            annotator,
            data_index,
            validator,
            sandbox_id=self.sandbox_id,
            introduced_by=agent_index,
            introduced_via=via,
        )
        if admitted is None:
            return None
        self._data[data_index] = admitted
        self._append(
            _mem_key_data(data_index),
            "introduced",
            agent_index,
            {"via": via, "data_index": data_index},
        )
        return admitted

    def apply_view(
        self,
        admitted: AdmittedMemory,
        *,
        agent_index: ImmutableIndex | None = None,
    ) -> AbstractMemory | None:
        if admitted.data_index not in self._data:
            return None
        node_index = self.allocate("view")
        node = apply_viewpoint(admitted, node_index=node_index)
        if node is None:
            return None
        self._nodes[node_index] = node
        self._append(
            _mem_key_node(node_index),
            "view_applied",
            agent_index,
            {"data_index": admitted.data_index, "node_index": node_index},
        )
        self._append(
            _mem_key_data(admitted.data_index),
            "view_derived",
            agent_index,
            {"node_index": node_index},
        )
        return node

    def abstract(
        self,
        prior: AbstractMemory,
        declare: Callable[..., Any],
        *,
        tags: MemoryTags | None = None,
        agent_index: ImmutableIndex | None = None,
    ) -> AbstractMemory | None:
        node_index = self.allocate("view")
        node = abstract_memory(prior, declare, index=node_index, tags=tags)
        if node is None:
            return None
        self._nodes[node_index] = node
        self._append(
            _mem_key_node(node_index),
            "abstract_lift",
            agent_index,
            {"prior": prior.index},
        )
        return node

    def reflect_merge(
        self,
        priors: list[StructuralNode] | tuple[StructuralNode, ...],
        viewpoint: ViewpointMemory,
        *,
        tags: MemoryTags | None = None,
        agent_index: ImmutableIndex | None = None,
    ) -> ReflectMemory | None:
        node_index = self.allocate("reflection")
        node = reflect_merge(priors, index=node_index, viewpoint=viewpoint, tags=tags)
        if node is None:
            return None
        self._nodes[node_index] = node
        self._append(
            _mem_key_node(node_index),
            "reflection_merge",
            agent_index,
            {
                "origins": [p.index for p in priors],
                "data_indexes": list(node.data_indexes),
            },
        )
        for p in priors:
            self._append(
                _mem_key_node(p.index),
                "merged_into",
                agent_index,
                {"new_node": node_index},
            )
        return node

    def reflect_forget(
        self,
        target: StructuralNode,
        viewpoint: ViewpointMemory,
        *,
        tags: MemoryTags | None = None,
        agent_index: ImmutableIndex | None = None,
    ) -> ReflectMemory | None:
        node_index = self.allocate("reflection")
        node = reflect_forget(target, index=node_index, viewpoint=viewpoint, tags=tags)
        if node is None:
            return None
        self._nodes[node_index] = node
        self._append(
            _mem_key_node(node_index),
            "reflection_forget",
            agent_index,
            {"target": target.index},
        )
        self._append(
            _mem_key_node(target.index),
            "forgotten_by",
            agent_index,
            {"tombstone": node_index},
        )
        return node

    def get_data(self, data_index: ImmutableIndex) -> AdmittedMemory | None:
        return self._data.get(data_index)

    def get_node(self, node_index: ImmutableIndex) -> StructuralNode | None:
        return self._nodes.get(node_index)

    def append_op(
        self,
        mem_key: str,
        name: str,
        agent_index: ImmutableIndex | None = None,
        detail: Any = None,
    ) -> OpEvent:
        return self._append(mem_key, name, agent_index, detail)

    def ops(self, mem_key: str) -> tuple[OpEvent, ...]:
        return tuple(self._ops.get(mem_key, ()))

    def ops_for_data(self, data_index: ImmutableIndex) -> tuple[OpEvent, ...]:
        return self.ops(_mem_key_data(data_index))

    def ops_for_node(self, node_index: ImmutableIndex) -> tuple[OpEvent, ...]:
        return self.ops(_mem_key_node(node_index))

    def _append(
        self,
        mem_key: str,
        name: str,
        agent_index: ImmutableIndex | None,
        detail: Any,
    ) -> OpEvent:
        seq = self._op_seq.get(mem_key, 0) + 1
        self._op_seq[mem_key] = seq
        event = OpEvent(seq=seq, name=name, agent_index=agent_index, detail=detail)
        self._ops.setdefault(mem_key, []).append(event)
        return event
