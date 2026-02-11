import os
import random
import chess
import numpy as np
from tqdm import tqdm
from concurrent.futures import ProcessPoolExecutor, as_completed

from config import DATA_DIR, MAX_POSITIONS_PER_GAME, MIN_PIECES
from features import board_to_features, piece_count

PIECE_VALUES = {
    chess.PAWN: 100, chess.KNIGHT: 320, chess.BISHOP: 330,
    chess.ROOK: 500, chess.QUEEN: 900, chess.KING: 20000,
}


def quiescence_eval(board, alpha=-10000, beta=10000):
    score = sum(
        (1 if p.color == chess.WHITE else -1) * PIECE_VALUES[p.piece_type]
        for sq in chess.SQUARES if (p := board.piece_at(sq))
    )
    if board.turn == chess.BLACK:
        score = -score
    
    if score >= beta:
        return beta
    alpha = max(alpha, score)
    
    for move in board.legal_moves:
        if not board.is_capture(move):
            continue
        board.push(move)
        eval_score = -quiescence_eval(board, -beta, -alpha)
        board.pop()
        if eval_score >= beta:
            return beta
        alpha = max(alpha, eval_score)
    
    return alpha


def evaluate_position(board):
    score = quiescence_eval(board)
    return -score if board.turn == chess.BLACK else score


def generate_game_data():
    data = []
    board = chess.Board()
    move_count = 0
    
    while not board.is_game_over() and len(data) < MAX_POSITIONS_PER_GAME:
        if move_count >= 8 and piece_count(board) >= MIN_PIECES:
            eval_score = max(-2000, min(2000, evaluate_position(board)))
            white_feat, black_feat, stm = board_to_features(board)
            data.append((white_feat, black_feat, stm, float(eval_score)))
        
        legal_moves = list(board.legal_moves)
        if not legal_moves:
            break
        
        if random.random() < 0.3:
            preferred = [m for m in legal_moves if board.is_capture(m) or board.gives_check(m)]
            move = random.choice(preferred) if preferred else random.choice(legal_moves)
        else:
            move = random.choice(legal_moves)
        
        board.push(move)
        move_count += 1
    
    return data


def generate_batch_data(num_games, num_workers=4):
    all_data = []
    
    with ProcessPoolExecutor(max_workers=num_workers) as executor:
        futures = [executor.submit(generate_game_data) for _ in range(num_games)]
        for future in tqdm(as_completed(futures), total=num_games, desc="Generating"):
            all_data.extend(future.result())
    
    if not all_data:
        raise ValueError("No data generated!")
    
    return (
        np.array([d[0] for d in all_data], dtype=np.float32),
        np.array([d[1] for d in all_data], dtype=np.float32),
        np.array([[d[2]] for d in all_data], dtype=np.float32),
        np.array([[d[3]] for d in all_data], dtype=np.float32),
    )


def save_data(white_features, black_features, stm, evals, filename):
    os.makedirs(DATA_DIR, exist_ok=True)
    np.savez_compressed(
        os.path.join(DATA_DIR, filename),
        white_features=white_features, black_features=black_features, stm=stm, evals=evals
    )


def load_data(filename):
    data = np.load(os.path.join(DATA_DIR, filename))
    return data['white_features'], data['black_features'], data['stm'], data['evals']


if __name__ == "__main__":
    w, b, s, e = generate_batch_data(100, num_workers=4)
    print(f"Generated {len(e)} positions, eval range: [{e.min():.0f}, {e.max():.0f}]")
    save_data(w, b, s, e, "sample_data.npz")
