import chess
import numpy as np
from config import INPUT_SIZE

PIECE_INDICES = {
    chess.PAWN: 0, chess.KNIGHT: 1, chess.BISHOP: 2,
    chess.ROOK: 3, chess.QUEEN: 4, chess.KING: 5,
}


def get_feature_index(piece_type, piece_color, square, perspective):
    if not perspective:
        square = square ^ 56
    piece_idx = PIECE_INDICES[piece_type]
    if piece_color != perspective:
        piece_idx += 6
    return square * 12 + piece_idx


def board_to_features(board):
    white_features = np.zeros(INPUT_SIZE, dtype=np.float32)
    black_features = np.zeros(INPUT_SIZE, dtype=np.float32)
    
    for square in chess.SQUARES:
        piece = board.piece_at(square)
        if piece is not None:
            white_idx = get_feature_index(piece.piece_type, piece.color, square, True)
            black_idx = get_feature_index(piece.piece_type, piece.color, square, False)
            white_features[white_idx] = 1.0
            black_features[black_idx] = 1.0
    
    stm_white = 1.0 if board.turn == chess.WHITE else 0.0
    return white_features, black_features, stm_white


def piece_count(board):
    return bin(board.occupied).count('1')


if __name__ == "__main__":
    board = chess.Board()
    w, b, stm = board_to_features(board)
    print(f"Features: white={np.count_nonzero(w)}, black={np.count_nonzero(b)}, stm={stm}")
