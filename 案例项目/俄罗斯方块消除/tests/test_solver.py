import unittest

from gamebot.model import Board, Piece
from gamebot.solver import Solver, SolverConfig


class SolverTests(unittest.TestCase):
    def test_finds_clear_before_larger_piece(self):
        board = Board.from_matrix(
            (
                (1, 1, 1, 0),
                (1, 1, 0, 0),
                (0, 1, 0, 1),
                (1, 0, 1, 0),
            )
        )
        single = Piece(((0, 0),), "single")
        square = Piece(((0, 0), (0, 1), (1, 0), (1, 1)), "square")
        plan = Solver(SolverConfig(candidate_limit=16)).solve(board, (square, single))
        self.assertEqual(plan.placed_count, 2)
        self.assertEqual(plan.first_move.piece_index, 1)

    def test_returns_empty_plan_for_dead_board(self):
        board = Board.from_matrix(((1, 1), (1, 0)))
        square = Piece(((0, 0), (0, 1), (1, 0), (1, 1)), "square")
        plan = Solver().solve(board, (square,))
        self.assertEqual(plan.placed_count, 0)
        self.assertEqual(plan.moves, ())

    def test_duplicate_pieces_still_produce_two_moves(self):
        board = Board(3, 3)
        single = Piece(((0, 0),), "single")
        plan = Solver().solve(board, (single, single))
        self.assertEqual(plan.placed_count, 2)
        self.assertEqual({move.piece_index for move in plan.moves}, {0, 1})


if __name__ == "__main__":
    unittest.main()
