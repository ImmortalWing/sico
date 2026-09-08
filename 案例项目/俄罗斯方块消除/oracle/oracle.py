#!/usr/bin/env python3
"""Fixed comparison oracle for the M14 offline block-game solver corpus.

RFC-0038 §3.1: Python serves only as the fixed comparison oracle. This
script implements the integer-arithmetic solver that the Sico port
(`tests/end-to-end/block-solver.sico`) mirrors instruction for instruction;
both sides must produce byte-identical stdout for every frozen fixture.

Algorithm contract (integer scale, weights = solver.py weights x 100 where
applicable):
  - 8x8 bitboard in one u64, bit i = cell (i // 8, i % 8).
  - board text: 64 chars, '#' occupied, 'T' occupied + target, '.' empty.
  - tray text: pieces separated by '|', piece rows by ';', row chars '#/.'.
  - placement: normalized piece mask shifted by (r*8+c); bounds + collision.
  - clearing: simultaneous full rows and columns; targets cleared counted
    from the target mask.
  - transition reward: 12000*lines^2 + 26000*targets_cleared.
  - leaf evaluation: reward - 150*occupied + 1400*largest_empty_region
    + (row_pot4 + col_pot4) * 14000 // 4096 - 2600*targets_remaining,
    where pot4 sums count^4 per row and column (count 0..8).
  - search: depth-first over tray pieces, duplicate shapes skipped at each
    level (by mask+dims, comparing only earlier unused candidates), origins
    scanned row-major. Node budget counts every enumerated (piece, r, c)
    candidate; exceeding MAX_NODES sets the typed limit flag and unwinds.
  - dead branch (no legal placement at some level): score MIN sentinel.
  - result: best leaf by strictly-greater score (first found wins ties);
    if no leaf exists, placed = 0 and score = eval(initial board).
  - output: one JSON object line:
    {"placed":P,"score":S,"moves":"pi:r,c;...","limit":B}

Usage: python oracle.py < input.json   (prints the result line)
"""
from __future__ import annotations

import json
import sys

ROWS = 8
COLS = 8
FULL = (1 << 64) - 1
MAX_NODES = 200000
MIN_SCORE = -1_000_000_000


def popcount(x: int) -> int:
    count = 0
    for _ in range(64):
        count += x & 1
        x >>= 1
    return count


def largest_empty_region(occ: int) -> int:
    empty = FULL ^ occ
    visited = 0
    largest = 0
    while empty ^ visited:
        remaining = empty ^ visited
        start = 1
        index = 0
        while remaining & start == 0:
            start <<= 1
            index += 1
        frontier = start
        region = 0
        while frontier:
            region |= frontier
            neighbours = (
                ((frontier << 1) & 0x7F7F7F7F7F7F7F7F)
                | (frontier >> 1)
                | ((frontier << 8) & FULL)
                | (frontier >> 8)
            )
            frontier = neighbours & empty & (FULL ^ region)
        visited |= region
        size = popcount(region)
        if size > largest:
            largest = size
    return largest


def evaluate(occ: int, targets: int, reward: int) -> int:
    row_pot4 = 0
    for r in range(ROWS):
        row_mask = 0xFF << (8 * r)
        count = popcount(occ & row_mask)
        row_pot4 += count * count * count * count
    col_pot4 = 0
    for c in range(COLS):
        col_mask = 0
        for k in range(ROWS):
            col_mask |= 1 << (8 * k + c)
        count = popcount(occ & col_mask)
        col_pot4 += count * count * count * count
    pot_term = (row_pot4 + col_pot4) * 14000 // 4096
    return (
        reward
        - 150 * popcount(occ)
        + 1400 * largest_empty_region(occ)
        + pot_term
        - 2600 * popcount(targets)
    )


def place(occ: int, targets: int, mask: int, row: int, col: int):
    shifted = mask << (row * 8 + col)
    occupied = occ | shifted
    cleared = 0
    lines = 0
    for r in range(ROWS):
        row_mask = 0xFF << (8 * r)
        if occupied & row_mask == row_mask:
            cleared |= row_mask
            lines += 1
    for c in range(COLS):
        col_mask = 0
        for k in range(ROWS):
            col_mask |= 1 << (8 * k + c)
        if occupied & col_mask == col_mask:
            cleared |= col_mask
            lines += 1
    cleared_targets = popcount(targets & cleared)
    occupied &= FULL ^ cleared
    targets &= FULL ^ cleared
    reward = 12000 * lines * lines + 26000 * cleared_targets
    return occupied, targets, reward


class Plan:
    __slots__ = ("score", "moves", "placed", "nodes", "limit")

    def __init__(self, score, moves, placed, nodes, limit):
        self.score = score
        self.moves = moves
        self.placed = placed
        self.nodes = nodes
        self.limit = limit


def solve(occ: int, targets: int, tray) -> Plan:
    tray = [(mask, height, width) for (mask, height, width) in tray]

    def visit(level, used, occ, targets, reward):
        nodes = 0
        limit = False
        if level == len(tray):
            return Plan(evaluate(occ, targets, reward), "", 0, 0, False)
        best = Plan(MIN_SCORE, "", 0, 0, False)
        for pi in range(len(tray)):
            if used & (1 << pi):
                continue
            mask, height, width = tray[pi]
            duplicate = False
            for pj in range(pi):
                if used & (1 << pj):
                    continue
                other = tray[pj]
                if other[0] == mask and other[1] == height and other[2] == width:
                    duplicate = True
                    break
            if duplicate:
                continue
            for row in range(ROWS - height + 1):
                for col in range(COLS - width + 1):
                    nodes += 1
                    if nodes > MAX_NODES:
                        limit = True
                        break
                    if occ & (mask << (row * 8 + col)):
                        continue
                    occ2, targets2, reward2 = place(occ, targets, mask, row, col)
                    child = visit(
                        level + 1,
                        used | (1 << pi),
                        occ2,
                        targets2,
                        reward + reward2,
                    )
                    nodes += child.nodes
                    moved = f"{pi}:{row},{col}"
                    joined = moved if child.moves == "" else moved + ";" + child.moves
                    if child.score > best.score:
                        best = Plan(child.score, joined, child.placed + 1, 0, False)
                    limit = limit or child.limit
                    if limit:
                        break
                if limit:
                    break
            if limit:
                break
        return Plan(best.score, best.moves, best.placed, nodes, limit)

    result = visit(0, 0, occ, targets, 0)
    if result.score <= MIN_SCORE:
        return Plan(evaluate(occ, targets, 0), "", 0, 0, result.limit)
    return result


def parse_board(text: str):
    occ = 0
    targets = 0
    if len(text) != 64 or any(ch not in "#T." for ch in text):
        raise ValueError("board must be 64 chars of '#', 'T', '.'")
    for index, ch in enumerate(text):
        if ch in "#T":
            occ |= 1 << index
        if ch == "T":
            targets |= 1 << index
    return occ, targets


def parse_tray(text: str):
    tray = []
    for piece_text in text.split("|"):
        rows = piece_text.split(";")
        if not piece_text or any(ch not in "#." for row in rows for ch in row) or not all(rows):
            raise ValueError("tray piece rows must be non-empty '#/.' rows")
        width = max(len(row) for row in rows)
        mask = 0
        for dr, row in enumerate(rows):
            for dc, ch in enumerate(row):
                if ch == "#":
                    mask |= 1 << (dr * 8 + dc)
        tray.append((mask, len(rows), width))
    return tray


def main() -> int:
    document = json.load(sys.stdin)
    occ, targets = parse_board(document["board"])
    tray = parse_tray(document["tray"])
    plan = solve(occ, targets, tray)
    print(
        json.dumps(
            {
                "placed": plan.placed,
                "score": plan.score,
                "moves": plan.moves,
                "limit": plan.limit,
            },
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
