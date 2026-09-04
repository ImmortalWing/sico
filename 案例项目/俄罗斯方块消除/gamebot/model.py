from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from typing import Iterable, Mapping, Sequence


Coord = tuple[int, int]


def _normalise_cells(cells: Iterable[Coord]) -> tuple[Coord, ...]:
    unique = set(cells)
    if not unique:
        raise ValueError("图形至少需要一个格子")
    min_r = min(r for r, _ in unique)
    min_c = min(c for _, c in unique)
    return tuple(sorted((r - min_r, c - min_c) for r, c in unique))


@dataclass(frozen=True, slots=True)
class Piece:
    """固定朝向的待放图形；tags 可描述蛋、宝石等特殊格。"""

    cells: tuple[Coord, ...]
    name: str = "piece"
    tags: tuple[tuple[int, int, str], ...] = ()

    def __post_init__(self) -> None:
        original = tuple(self.cells)
        normalised = _normalise_cells(original)
        if normalised != tuple(sorted(original)):
            # 同步移动 tag，使图形左上角为 (0, 0)。
            min_r = min(r for r, _ in original)
            min_c = min(c for _, c in original)
            shifted_tags = tuple(sorted((r - min_r, c - min_c, tag) for r, c, tag in self.tags))
            object.__setattr__(self, "tags", shifted_tags)
        object.__setattr__(self, "cells", normalised)
        if any((r, c) not in normalised for r, c, _ in self.tags):
            raise ValueError("特殊标签必须位于图形格子内")

    @property
    def height(self) -> int:
        return max(r for r, _ in self.cells) + 1

    @property
    def width(self) -> int:
        return max(c for _, c in self.cells) + 1

    @property
    def signature(self) -> tuple[tuple[Coord, ...], tuple[tuple[int, int, str], ...]]:
        return self.cells, self.tags


@dataclass(frozen=True, slots=True)
class Move:
    piece_index: int
    row: int
    col: int


@dataclass(frozen=True, slots=True)
class Transition:
    board: "Board"
    cleared_rows: tuple[int, ...]
    cleared_cols: tuple[int, ...]
    cleared_targets: tuple[tuple[str, int], ...]
    placed_cells: tuple[Coord, ...]

    @property
    def cleared_line_count(self) -> int:
        return len(self.cleared_rows) + len(self.cleared_cols)

    def target_count(self, name: str | None = None) -> int:
        counts = dict(self.cleared_targets)
        return sum(counts.values()) if name is None else counts.get(name, 0)


@dataclass(frozen=True, slots=True)
class Board:
    rows: int = 8
    cols: int = 8
    occupied: int = 0
    # (linear cell index, target name)，只保存当前仍在棋盘上的目标。
    targets: tuple[tuple[int, str], ...] = ()

    def __post_init__(self) -> None:
        if self.rows <= 0 or self.cols <= 0:
            raise ValueError("棋盘尺寸必须为正数")
        valid_mask = (1 << (self.rows * self.cols)) - 1
        if self.occupied & ~valid_mask:
            raise ValueError("occupied 含有棋盘范围外的位")
        ordered = tuple(sorted(self.targets))
        if ordered != self.targets:
            object.__setattr__(self, "targets", ordered)
        for index, _ in self.targets:
            if not (0 <= index < self.rows * self.cols):
                raise ValueError("目标格超出棋盘")
            if not (self.occupied >> index) & 1:
                raise ValueError("目标格必须同时是占用格")

    @classmethod
    def from_matrix(
        cls,
        matrix: Sequence[Sequence[int | bool]],
        targets: Mapping[Coord, str] | None = None,
    ) -> "Board":
        if not matrix or not matrix[0]:
            raise ValueError("棋盘矩阵不能为空")
        rows, cols = len(matrix), len(matrix[0])
        if any(len(row) != cols for row in matrix):
            raise ValueError("棋盘矩阵每行长度必须一致")
        occupied = 0
        for r, row in enumerate(matrix):
            for c, value in enumerate(row):
                if value:
                    occupied |= 1 << (r * cols + c)
        encoded_targets = tuple(sorted((r * cols + c, tag) for (r, c), tag in (targets or {}).items()))
        return cls(rows, cols, occupied, encoded_targets)

    def is_occupied(self, row: int, col: int) -> bool:
        return bool(self.occupied & (1 << (row * self.cols + col)))

    def matrix(self) -> tuple[tuple[int, ...], ...]:
        return tuple(
            tuple(int(self.is_occupied(r, c)) for c in range(self.cols))
            for r in range(self.rows)
        )

    def target_map(self) -> dict[Coord, str]:
        return {(index // self.cols, index % self.cols): tag for index, tag in self.targets}

    @property
    def occupied_count(self) -> int:
        return self.occupied.bit_count()

    def can_place(self, piece: Piece, row: int, col: int) -> bool:
        if row < 0 or col < 0 or row + piece.height > self.rows or col + piece.width > self.cols:
            return False
        return all(not self.is_occupied(row + dr, col + dc) for dr, dc in piece.cells)

    def legal_origins(self, piece: Piece) -> tuple[Coord, ...]:
        return tuple(
            (r, c)
            for r in range(self.rows - piece.height + 1)
            for c in range(self.cols - piece.width + 1)
            if self.can_place(piece, r, c)
        )

    def place(self, piece: Piece, row: int, col: int) -> Transition:
        if not self.can_place(piece, row, col):
            raise ValueError(f"非法落点: {piece.name} @ ({row}, {col})")

        placed = tuple((row + dr, col + dc) for dr, dc in piece.cells)
        occupied = self.occupied
        for r, c in placed:
            occupied |= 1 << (r * self.cols + c)

        target_map = {index: tag for index, tag in self.targets}
        for dr, dc, tag in piece.tags:
            target_map[(row + dr) * self.cols + col + dc] = tag

        full_rows = tuple(
            r for r in range(self.rows)
            if all(occupied & (1 << (r * self.cols + c)) for c in range(self.cols))
        )
        full_cols = tuple(
            c for c in range(self.cols)
            if all(occupied & (1 << (r * self.cols + c)) for r in range(self.rows))
        )
        cleared_indices = {
            r * self.cols + c
            for r in full_rows
            for c in range(self.cols)
        } | {
            r * self.cols + c
            for c in full_cols
            for r in range(self.rows)
        }
        for index in cleared_indices:
            occupied &= ~(1 << index)

        cleared_targets = Counter(target_map[index] for index in cleared_indices if index in target_map)
        remaining_targets = tuple(sorted((index, tag) for index, tag in target_map.items() if index not in cleared_indices))
        next_board = Board(self.rows, self.cols, occupied, remaining_targets)
        return Transition(
            board=next_board,
            cleared_rows=full_rows,
            cleared_cols=full_cols,
            cleared_targets=tuple(sorted(cleared_targets.items())),
            placed_cells=placed,
        )

    def __str__(self) -> str:
        targets = self.target_map()
        lines = []
        for r in range(self.rows):
            lines.append(" ".join(targets.get((r, c), "#") if self.is_occupied(r, c) else "." for c in range(self.cols)))
        return "\n".join(lines)

