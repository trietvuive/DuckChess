"""Dataset classes for NNUE training"""

import os
import chess
import chess.pgn
import numpy as np
import torch
from torch.utils.data import Dataset, IterableDataset
from typing import Optional, List, Tuple
from tqdm import tqdm

from config import INPUT_SIZE, DATA_DIR, MIN_PIECES
from features import board_to_features, piece_count


class NNUEDataset(Dataset):
    """Standard dataset for loading pre-generated data"""
    
    def __init__(self, data_file: str):
        data_path = os.path.join(DATA_DIR, data_file)
        data = np.load(data_path)
        
        self.white_features = torch.from_numpy(data['white_features'])
        self.black_features = torch.from_numpy(data['black_features'])
        self.stm = torch.from_numpy(data['stm'])
        self.evals = torch.from_numpy(data['evals']) / 1000.0  # Normalize
    
    def __len__(self):
        return len(self.evals)
    
    def __getitem__(self, idx):
        return (
            self.white_features[idx],
            self.black_features[idx],
            self.stm[idx],
            self.evals[idx]
        )


class PGNDataset(IterableDataset):
    """
    Iterable dataset that streams positions from PGN files
    
    Positions are labeled using game results and stockfish-like heuristics
    """
    
    def __init__(self, pgn_paths: List[str], max_positions: Optional[int] = None):
        self.pgn_paths = pgn_paths
        self.max_positions = max_positions
    
    def __iter__(self):
        positions_yielded = 0
        
        for pgn_path in self.pgn_paths:
            with open(pgn_path) as pgn_file:
                while True:
                    game = chess.pgn.read_game(pgn_file)
                    if game is None:
                        break
                    
                    # Get game result for labeling
                    result = game.headers.get("Result", "*")
                    if result == "1-0":
                        game_eval = 1.0  # White wins
                    elif result == "0-1":
                        game_eval = -1.0  # Black wins
                    elif result == "1/2-1/2":
                        game_eval = 0.0  # Draw
                    else:
                        continue  # Skip unfinished games
                    
                    # Iterate through game positions
                    board = game.board()
                    move_count = 0
                    total_moves = len(list(game.mainline_moves()))
                    
                    for move in game.mainline_moves():
                        board.push(move)
                        move_count += 1
                        
                        # Skip early opening and endgame
                        if move_count < 10 or piece_count(board) < MIN_PIECES:
                            continue
                        
                        # Create position-based eval (blend of result and material)
                        # Early positions use more material, late positions use more result
                        progress = move_count / max(total_moves, 1)
                        material = self._quick_eval(board) / 1000.0
                        
                        # Blend: early game = mostly material, late game = mostly result
                        position_eval = (1 - progress) * material + progress * game_eval
                        
                        # Convert to side-to-move perspective
                        if board.turn == chess.BLACK:
                            position_eval = -position_eval
                        
                        # Get features
                        white_feat, black_feat, stm = board_to_features(board)
                        
                        yield (
                            torch.from_numpy(white_feat),
                            torch.from_numpy(black_feat),
                            torch.tensor([stm], dtype=torch.float32),
                            torch.tensor([position_eval], dtype=torch.float32)
                        )
                        
                        positions_yielded += 1
                        if self.max_positions and positions_yielded >= self.max_positions:
                            return
    
    def _quick_eval(self, board: chess.Board) -> int:
        """Quick material evaluation"""
        piece_values = {
            chess.PAWN: 100, chess.KNIGHT: 320, chess.BISHOP: 330,
            chess.ROOK: 500, chess.QUEEN: 900, chess.KING: 0
        }
        
        score = 0
        for sq in chess.SQUARES:
            piece = board.piece_at(sq)
            if piece:
                value = piece_values[piece.piece_type]
                score += value if piece.color == chess.WHITE else -value
        return score


class StockfishDataset(IterableDataset):
    """
    Dataset using Stockfish evaluations for higher quality labels
    
    Requires stockfish to be installed and accessible
    """
    
    def __init__(
        self,
        pgn_paths: List[str],
        stockfish_path: str = "stockfish",
        depth: int = 10,
        max_positions: Optional[int] = None
    ):
        self.pgn_paths = pgn_paths
        self.stockfish_path = stockfish_path
        self.depth = depth
        self.max_positions = max_positions
    
    def __iter__(self):
        try:
            engine = chess.engine.SimpleEngine.popen_uci(self.stockfish_path)
        except Exception as e:
            raise RuntimeError(f"Could not start Stockfish: {e}")
        
        try:
            positions_yielded = 0
            
            for pgn_path in self.pgn_paths:
                with open(pgn_path) as pgn_file:
                    while True:
                        game = chess.pgn.read_game(pgn_file)
                        if game is None:
                            break
                        
                        board = game.board()
                        move_count = 0
                        
                        for move in game.mainline_moves():
                            board.push(move)
                            move_count += 1
                            
                            if move_count < 10 or piece_count(board) < MIN_PIECES:
                                continue
                            
                            # Get Stockfish evaluation
                            info = engine.analyse(board, chess.engine.Limit(depth=self.depth))
                            score = info["score"].white()
                            
                            # Convert to centipawns
                            if score.is_mate():
                                cp = 10000 if score.mate() > 0 else -10000
                            else:
                                cp = score.score()
                            
                            # Normalize
                            eval_normalized = np.clip(cp / 1000.0, -2.0, 2.0)
                            
                            # Get features
                            white_feat, black_feat, stm = board_to_features(board)
                            
                            yield (
                                torch.from_numpy(white_feat),
                                torch.from_numpy(black_feat),
                                torch.tensor([stm], dtype=torch.float32),
                                torch.tensor([eval_normalized], dtype=torch.float32)
                            )
                            
                            positions_yielded += 1
                            if self.max_positions and positions_yielded >= self.max_positions:
                                engine.quit()
                                return
        finally:
            engine.quit()


def convert_pgn_to_npz(
    pgn_paths: List[str],
    output_file: str,
    max_positions: Optional[int] = None,
    use_stockfish: bool = False,
    stockfish_path: str = "stockfish",
    stockfish_depth: int = 10
):
    """
    Convert PGN files to NPZ training data
    
    Args:
        pgn_paths: List of PGN file paths
        output_file: Output NPZ filename
        max_positions: Maximum positions to extract
        use_stockfish: Use Stockfish for evaluation labels
        stockfish_path: Path to Stockfish binary
        stockfish_depth: Stockfish search depth
    """
    white_features = []
    black_features = []
    stms = []
    evals = []
    
    if use_stockfish:
        dataset = StockfishDataset(pgn_paths, stockfish_path, stockfish_depth, max_positions)
    else:
        dataset = PGNDataset(pgn_paths, max_positions)
    
    print(f"Converting PGN files to training data...")
    
    for white_feat, black_feat, stm, eval_score in tqdm(dataset):
        white_features.append(white_feat.numpy())
        black_features.append(black_feat.numpy())
        stms.append(stm.numpy())
        evals.append(eval_score.numpy() * 1000)  # Store in centipawns
    
    # Save
    os.makedirs(DATA_DIR, exist_ok=True)
    output_path = os.path.join(DATA_DIR, output_file)
    
    np.savez_compressed(
        output_path,
        white_features=np.array(white_features, dtype=np.float32),
        black_features=np.array(black_features, dtype=np.float32),
        stm=np.array(stms, dtype=np.float32),
        evals=np.array(evals, dtype=np.float32)
    )
    
    print(f"Saved {len(evals)} positions to {output_path}")


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Convert PGN to training data")
    parser.add_argument("pgn_files", nargs="+", help="PGN files to convert")
    parser.add_argument("--output", type=str, default="pgn_data.npz", help="Output filename")
    parser.add_argument("--max-positions", type=int, default=None, help="Max positions")
    parser.add_argument("--use-stockfish", action="store_true", help="Use Stockfish for labels")
    parser.add_argument("--stockfish-path", type=str, default="stockfish", help="Stockfish path")
    parser.add_argument("--stockfish-depth", type=int, default=10, help="Stockfish depth")
    
    args = parser.parse_args()
    
    convert_pgn_to_npz(
        args.pgn_files,
        args.output,
        args.max_positions,
        args.use_stockfish,
        args.stockfish_path,
        args.stockfish_depth
    )
