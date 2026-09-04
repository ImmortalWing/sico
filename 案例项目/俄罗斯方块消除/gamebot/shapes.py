from __future__ import annotations

from .model import Piece


def piece(name: str, *rows: str) -> Piece:
    cells = tuple((r, c) for r, line in enumerate(rows) for c, ch in enumerate(line) if ch != ".")
    return Piece(cells, name=name)


# 常见固定朝向图形。视觉模块会直接生成实际图形；这个库用于评估未来兼容性。
STANDARD_PIECES: tuple[Piece, ...] = (
    piece("single", "#"),
    piece("h2", "##"),
    piece("v2", "#", "#"),
    piece("h3", "###"),
    piece("v3", "#", "#", "#"),
    piece("h4", "####"),
    piece("v4", "#", "#", "#", "#"),
    piece("h5", "#####"),
    piece("v5", "#", "#", "#", "#", "#"),
    piece("square2", "##", "##"),
    piece("square3", "###", "###", "###"),
    piece("l3_a", "#.", "##"),
    piece("l3_b", ".#", "##"),
    piece("l3_c", "##", "#."),
    piece("l3_d", "##", ".#"),
    piece("corner5_a", "#..", "#..", "###"),
    piece("corner5_b", "..#", "..#", "###"),
    piece("corner5_c", "###", "#..", "#.."),
    piece("corner5_d", "###", "..#", "..#"),
    piece("t4_up", "###", ".#."),
    piece("t4_down", ".#.", "###"),
    piece("t4_left", "#.", "##", "#."),
    piece("t4_right", ".#", "##", ".#"),
)


PIECE_BY_NAME = {item.name: item for item in STANDARD_PIECES}

