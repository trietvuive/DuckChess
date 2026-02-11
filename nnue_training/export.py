import os
import json
import struct
import argparse
import torch
import numpy as np

from config import INPUT_SIZE, HIDDEN1_SIZE, HIDDEN2_SIZE, WEIGHT_SCALE, CHECKPOINT_DIR, EXPORT_DIR
from model import NNUE


def quantize(weights, scale):
    return np.clip(weights * scale, -32767, 32767).astype(np.int16)


def export_binary(model, path):
    os.makedirs(EXPORT_DIR, exist_ok=True)
    with open(path, 'wb') as f:
        f.write(struct.pack('4sI', b'NNUE', 1))
        f.write(struct.pack('III', INPUT_SIZE, HIDDEN1_SIZE, HIDDEN2_SIZE))
        
        for name, param in [
            ('input', model.input_layer), ('hidden', model.hidden1), ('output', model.output)
        ]:
            w = quantize(param.weight.detach().cpu().numpy().T, WEIGHT_SCALE)
            b = quantize(param.bias.detach().cpu().numpy(), WEIGHT_SCALE)
            f.write(w.tobytes())
            f.write(b.tobytes())
    
    print(f"Exported: {path} ({os.path.getsize(path):,} bytes)")


def export_json(model, path):
    os.makedirs(EXPORT_DIR, exist_ok=True)
    weights = {
        'input_weights': model.input_layer.weight.detach().cpu().numpy().tolist(),
        'input_biases': model.input_layer.bias.detach().cpu().numpy().tolist(),
        'hidden_weights': model.hidden1.weight.detach().cpu().numpy().tolist(),
        'hidden_biases': model.hidden1.bias.detach().cpu().numpy().tolist(),
        'output_weights': model.output.weight.detach().cpu().numpy().tolist(),
        'output_bias': model.output.bias.detach().cpu().numpy().tolist(),
    }
    with open(path, 'w') as f:
        json.dump(weights, f)
    print(f"Exported: {path}")


def main(args):
    model = NNUE()
    ckpt_path = os.path.join(CHECKPOINT_DIR, args.checkpoint)
    
    if os.path.exists(ckpt_path):
        model.load_state_dict(torch.load(ckpt_path, map_location='cpu')['model_state_dict'])
        print(f"Loaded: {ckpt_path}")
    else:
        print(f"Warning: {ckpt_path} not found, using random weights")
    
    base = os.path.splitext(args.checkpoint)[0]
    if args.format in ('binary', 'all'):
        export_binary(model, os.path.join(EXPORT_DIR, f"{base}.nnue"))
    if args.format in ('json', 'all'):
        export_json(model, os.path.join(EXPORT_DIR, f"{base}.json"))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--checkpoint", default="best_model.pt")
    parser.add_argument("--format", choices=['binary', 'json', 'all'], default='all')
    main(parser.parse_args())
