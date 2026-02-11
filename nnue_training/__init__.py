"""NNUE Training Package for DuckChess Engine"""

from .config import *
from .model import NNUE, NNUELoss, count_parameters
from .features import board_to_features, get_feature_index
from .data_gen import generate_batch_data, save_data, load_data
from .dataset import NNUEDataset, PGNDataset, StockfishDataset

__version__ = "0.1.0"
