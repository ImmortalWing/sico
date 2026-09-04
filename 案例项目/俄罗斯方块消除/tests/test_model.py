import unittest

from gamebot.model import Board, Piece


class BoardTests(unittest.TestCase):
    def test_rejects_overlap_and_out_of_bounds(self):
        board = Board.from_matrix(((1, 0), (0, 0)))
        domino = Piece(((0, 0), (0, 1)), "domino")
        self.assertFalse(board.can_place(domino, 0, 0))
        self.assertFalse(board.can_place(domino, 1, 1))
        self.assertTrue(board.can_place(domino, 1, 0))

    def test_row_and_column_clear_simultaneously(self):
        board = Board.from_matrix(
            ((1, 0, 1), (0, 0, 1), (1, 1, 0)),
            {(0, 0): "star", (1, 2): "blue"},
        )
        result = board.place(Piece(((0, 0),), "single"), 0, 1)
        self.assertEqual(result.cleared_rows, (0,))
        self.assertEqual(result.cleared_cols, ())
        self.assertEqual(result.target_count("star"), 1)
        self.assertFalse(result.board.is_occupied(0, 0))
        self.assertTrue(result.board.is_occupied(1, 2))

        cross = Board.from_matrix(((1, 0, 1), (1, 1, 0), (1, 0, 1)))
        transition = cross.place(Piece(((0, 0),), "single"), 1, 2)
        self.assertEqual(transition.cleared_rows, (1,))
        self.assertEqual(transition.cleared_cols, (0, 2))
        self.assertEqual(transition.board.occupied_count, 0)

    def test_piece_normalisation_moves_tags(self):
        item = Piece(((4, 6), (5, 6)), "tagged", ((5, 6, "egg"),))
        self.assertEqual(item.cells, ((0, 0), (1, 0)))
        self.assertEqual(item.tags, ((1, 0, "egg"),))


if __name__ == "__main__":
    unittest.main()

