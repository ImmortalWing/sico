from __future__ import annotations

import heapq
import math
import random
from collections import Counter, deque
from dataclasses import dataclass, field
from typing import Iterable, Sequence

from .model import Board, Move, Piece, Transition
from .shapes import STANDARD_PIECES


@dataclass(frozen=True, slots=True)
class ScoreWeights:
    cleared_line: float = 120.0
    cleared_target: float = 260.0
    occupied_penalty: float = 1.5
    largest_empty_region: float = 1.8
    empty_components_penalty: float = 18.0
    dead_end_penalty: float = 9.0
    open_2x2: float = 2.0
    open_3x3: float = 5.0
    future_placement: float = 1.35
    rare_shape_safety: float = 4.0
    line_potential: float = 14.0


@dataclass(frozen=True, slots=True)
class SolverConfig:
    candidate_limit: int = 48
    max_nodes: int = 500_000
    future_samples: int = 0
    future_batches: int = 2
    random_seed: int = 20260904
    weights: ScoreWeights = field(default_factory=ScoreWeights)


@dataclass(frozen=True, slots=True)
class Plan:
    moves: tuple[Move, ...]
    score: float
    placed_count: int
    final_board: Board
    explored_nodes: int
    expected_future: float = 0.0

    @property
    def first_move(self) -> Move | None:
        return self.moves[0] if self.moves else None


class PieceDistribution:
    """可在线记录图形出现频率；拉普拉斯先验防止早期样本偏置。"""

    def __init__(self, pieces: Iterable[Piece] = STANDARD_PIECES, prior: float = 1.0) -> None:
        self._pieces: dict[tuple, Piece] = {}
        self._counts: Counter[tuple] = Counter()
        for item in pieces:
            self._pieces[item.signature] = item
            self._counts[item.signature] += prior

    def observe(self, pieces: Iterable[Piece]) -> None:
        for item in pieces:
            self._pieces[item.signature] = item
            self._counts[item.signature] += 1.0

    def sample(self, rng: random.Random, count: int = 3) -> tuple[Piece, ...]:
        signatures = tuple(self._pieces)
        weights = tuple(self._counts[sig] for sig in signatures)
        selected = rng.choices(signatures, weights=weights, k=count)
        return tuple(self._pieces[sig] for sig in selected)

    @property
    def pieces(self) -> tuple[Piece, ...]:
        return tuple(self._pieces.values())


def _empty_components(board: Board) -> tuple[int, int, int]:
    unseen = {(r, c) for r in range(board.rows) for c in range(board.cols) if not board.is_occupied(r, c)}
    components = 0
    largest = 0
    dead_ends = 0
    while unseen:
        components += 1
        start = unseen.pop()
        queue = deque([start])
        size = 0
        while queue:
            r, c = queue.popleft()
            size += 1
            neighbours = [
                (nr, nc)
                for nr, nc in ((r - 1, c), (r + 1, c), (r, c - 1), (r, c + 1))
                if 0 <= nr < board.rows and 0 <= nc < board.cols and not board.is_occupied(nr, nc)
            ]
            if len(neighbours) <= 1:
                dead_ends += 1
            for pos in neighbours:
                if pos in unseen:
                    unseen.remove(pos)
                    queue.append(pos)
        largest = max(largest, size)
    return components, largest, dead_ends


def _open_rectangles(board: Board, height: int, width: int) -> int:
    return sum(
        all(not board.is_occupied(r + dr, c + dc) for dr in range(height) for dc in range(width))
        for r in range(board.rows - height + 1)
        for c in range(board.cols - width + 1)
    )


class Solver:
    def __init__(
        self,
        config: SolverConfig | None = None,
        distribution: PieceDistribution | None = None,
    ) -> None:
        self.config = config or SolverConfig()
        self.distribution = distribution or PieceDistribution()

    def evaluate_board(self, board: Board) -> float:
        w = self.config.weights
        components, largest, dead_ends = _empty_components(board)
        placement_counts = [len(board.legal_origins(item)) for item in self.distribution.pieces]
        flexibility = sum(math.log1p(count) for count in placement_counts)
        # 最难容纳的若干图形决定临近死局时的安全性。
        rare_shape_safety = sum(sorted(placement_counts)[: max(1, len(placement_counts) // 5)])
        row_fill = [sum(board.is_occupied(r, c) for c in range(board.cols)) / board.cols for r in range(board.rows)]
        col_fill = [sum(board.is_occupied(r, c) for r in range(board.rows)) / board.rows for c in range(board.cols)]
        line_potential = sum(x**4 for x in row_fill + col_fill)
        return (
            -w.occupied_penalty * board.occupied_count
            + w.largest_empty_region * largest
            - w.empty_components_penalty * max(0, components - 1)
            - w.dead_end_penalty * dead_ends
            + w.open_2x2 * _open_rectangles(board, 2, 2)
            + w.open_3x3 * _open_rectangles(board, 3, 3)
            + w.future_placement * flexibility
            + w.rare_shape_safety * rare_shape_safety
            + w.line_potential * line_potential
            - w.cleared_target * len(board.targets) * 0.10
        )

    def quick_evaluate(self, board: Board) -> float:
        """搜索树叶子的低成本预评分；精细拓扑分析只用于入围候选。"""
        w = self.config.weights
        row_counts = [sum(board.is_occupied(r, c) for c in range(board.cols)) for r in range(board.rows)]
        col_counts = [sum(board.is_occupied(r, c) for r in range(board.rows)) for c in range(board.cols)]
        line_potential = sum((count / board.cols) ** 4 for count in row_counts)
        line_potential += sum((count / board.rows) ** 4 for count in col_counts)
        return (
            -w.occupied_penalty * board.occupied_count
            + w.line_potential * line_potential
            - w.cleared_target * len(board.targets) * 0.10
        )

    def transition_reward(self, transition: Transition) -> float:
        w = self.config.weights
        # 多行同时消除给额外奖励。
        lines = transition.cleared_line_count
        return w.cleared_line * lines * lines + w.cleared_target * transition.target_count()

    def solve(self, board: Board, tray: Sequence[Piece]) -> Plan:
        if not tray:
            return Plan((), self.evaluate_board(board), 0, board, 0)

        nodes = 0
        serial = 0
        # heap 保存确定性评分最高的叶子，再只对这些候选做随机后续评估。
        leaves: list[tuple[float, int, tuple[Move, ...], Board, float]] = []
        best_partial: tuple[int, float, tuple[Move, ...], Board] = (0, self.quick_evaluate(board), (), board)
        seen: dict[tuple[int, tuple[int, ...], tuple[tuple[int, str], ...]], float] = {}

        def add_leaf(moves: tuple[Move, ...], state: Board, reward: float) -> None:
            nonlocal serial
            deterministic = reward + self.quick_evaluate(state)
            serial += 1
            item = (deterministic, serial, moves, state, reward)
            if len(leaves) < self.config.candidate_limit:
                heapq.heappush(leaves, item)
            elif deterministic > leaves[0][0]:
                heapq.heapreplace(leaves, item)

        def visit(state: Board, remaining: tuple[int, ...], moves: tuple[Move, ...], reward: float) -> None:
            nonlocal nodes, best_partial
            if nodes >= self.config.max_nodes:
                return
            placed_count = len(moves)
            partial_score = reward + self.quick_evaluate(state)
            if placed_count > best_partial[0] or (placed_count == best_partial[0] and partial_score > best_partial[1]):
                best_partial = (placed_count, partial_score, moves, state)
            if not remaining:
                add_leaf(moves, state, reward)
                return

            cache_key = (state.occupied, remaining, state.targets)
            if seen.get(cache_key, float("-inf")) >= reward:
                return
            seen[cache_key] = reward

            duplicate_signatures: set[tuple] = set()
            progressed = False
            for position, piece_index in enumerate(remaining):
                item = tray[piece_index]
                if item.signature in duplicate_signatures:
                    continue
                duplicate_signatures.add(item.signature)
                next_remaining = remaining[:position] + remaining[position + 1 :]
                for row, col in state.legal_origins(item):
                    nodes += 1
                    progressed = True
                    transition = state.place(item, row, col)
                    visit(
                        transition.board,
                        next_remaining,
                        moves + (Move(piece_index, row, col),),
                        reward + self.transition_reward(transition),
                    )
                    if nodes >= self.config.max_nodes:
                        return
            if not progressed:
                # 无法放完托盘时也保留最深的方案，便于诊断死局。
                return

        visit(board, tuple(range(len(tray))), (), 0.0)
        if not leaves:
            placed, score, moves, state = best_partial
            return Plan(moves, score + self.evaluate_board(state), placed, state, nodes)

        candidates = sorted(leaves, reverse=True)
        if self.config.future_samples <= 0:
            scored = [
                (reward + self.evaluate_board(state), serial_id, moves, state)
                for _, serial_id, moves, state, reward in candidates
            ]
            score, _, moves, state = max(scored)
            return Plan(moves, score, len(moves), state, nodes)

        rng = random.Random(self.config.random_seed)
        best: Plan | None = None
        for _, _, moves, state, reward in candidates:
            deterministic = reward + self.evaluate_board(state)
            expected = self._rollout_value(state, rng)
            total = deterministic + expected
            candidate = Plan(moves, total, len(moves), state, nodes, expected)
            if best is None or candidate.score > best.score:
                best = candidate
        assert best is not None
        return best

    def _rollout_value(self, board: Board, rng: random.Random) -> float:
        total = 0.0
        for _ in range(self.config.future_samples):
            state = board
            survived = 0
            for _batch in range(self.config.future_batches):
                tray = list(self.distribution.sample(rng, 3))
                # rollout 使用逐步贪心，避免在候选叶子上再次进行完整树搜索。
                while tray:
                    best_choice: tuple[float, int, Board] | None = None
                    for index, item in enumerate(tray):
                        for row, col in state.legal_origins(item):
                            transition = state.place(item, row, col)
                            value = self.transition_reward(transition) + self.evaluate_board(transition.board)
                            if best_choice is None or value > best_choice[0]:
                                best_choice = (value, index, transition.board)
                    if best_choice is None:
                        break
                    _, index, state = best_choice
                    tray.pop(index)
                    survived += 1
            total += survived * 40.0 + self.evaluate_board(state) * 0.25
        return total / self.config.future_samples
