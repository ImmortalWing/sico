"""微信方块消除：规则模型、搜索、视觉识别与操作适配器。"""

from .model import Board, Move, Piece, Transition
from .solver import Plan, Solver, SolverConfig

__all__ = ["Board", "Move", "Piece", "Transition", "Plan", "Solver", "SolverConfig"]

