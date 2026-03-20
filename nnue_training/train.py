import os
import argparse

import lightning as L
import torch
from lightning.pytorch.callbacks import ModelCheckpoint
from torch.utils.data import DataLoader, random_split

from config import BATCH_SIZE, LEARNING_RATE, NUM_EPOCHS, VALIDATION_SPLIT, GAMES_PER_EPOCH, CHECKPOINT_DIR
from duck_nnue import count_parameters
from lit_module import DuckNNUELightningModule
from data_gen import generate_batch_data, save_data, load_data


def create_dataloaders(white_feat, black_feat, stm, evals, batch_size, val_split=0.1):
    dataset = torch.utils.data.TensorDataset(
        torch.from_numpy(white_feat),
        torch.from_numpy(black_feat),
        torch.from_numpy(stm),
        torch.from_numpy(evals) / 1000.0,
    )
    val_size = int(len(dataset) * val_split)
    train_ds, val_ds = random_split(
        dataset, [len(dataset) - val_size, val_size],
        generator=torch.Generator().manual_seed(42),
    )
    return (
        DataLoader(
            train_ds,
            batch_size=batch_size,
            shuffle=True,
            num_workers=0,
            pin_memory=True,
        ),
        DataLoader(
            val_ds,
            batch_size=batch_size,
            shuffle=False,
            num_workers=0,
            pin_memory=True,
        ),
    )


def main(args):
    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"Device: {device}")

    if args.data_file and os.path.exists(os.path.join("data", args.data_file)):
        white_feat, black_feat, stm, evals = load_data(args.data_file)
    else:
        white_feat, black_feat, stm, evals = generate_batch_data(args.num_games, args.workers)
        save_data(white_feat, black_feat, stm, evals, "training_data.npz")

    train_loader, val_loader = create_dataloaders(
        white_feat, black_feat, stm, evals, args.batch_size, VALIDATION_SPLIT
    )
    print(f"Train: {len(train_loader.dataset)}, Val: {len(val_loader.dataset)}")

    os.makedirs(CHECKPOINT_DIR, exist_ok=True)
    lit = DuckNNUELightningModule(
        lr=args.lr, max_epochs=args.epochs, lambda_reg=0.0001
    )
    print(f"Parameters: {count_parameters(lit.model):,}")

    checkpoint_cb = ModelCheckpoint(
        dirpath=CHECKPOINT_DIR,
        filename="best",
        monitor="val_mse",
        mode="min",
        save_top_k=1,
        save_last=True,
    )
    trainer = L.Trainer(
        max_epochs=args.epochs,
        accelerator="auto",
        devices=1,
        gradient_clip_val=1.0,
        enable_progress_bar=True,
        callbacks=[checkpoint_cb],
        log_every_n_steps=50,
    )
    trainer.fit(lit, train_loader, val_loader)

    # Legacy .pt for export.py (best weights from Lightning checkpoint)
    best_ckpt = checkpoint_cb.best_model_path
    if best_ckpt and os.path.isfile(best_ckpt):
        ck = torch.load(best_ckpt, map_location="cpu", weights_only=False)
        raw = ck.get("state_dict", ck)
        prefix = "model."
        sd = {k[len(prefix) :]: v for k, v in raw.items() if k.startswith(prefix)}
        if sd:
            lit.model.load_state_dict(sd)
    torch.save(
        {"model_state_dict": lit.model.state_dict(), "epoch": args.epochs - 1},
        os.path.join(CHECKPOINT_DIR, "best_model.pt"),
    )
    print(f"Wrote {CHECKPOINT_DIR}/best_model.pt")
    print("Done.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--epochs", type=int, default=NUM_EPOCHS)
    parser.add_argument("--batch-size", type=int, default=BATCH_SIZE)
    parser.add_argument("--lr", type=float, default=LEARNING_RATE)
    parser.add_argument("--num-games", type=int, default=GAMES_PER_EPOCH)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--data-file", type=str, default=None)
    main(parser.parse_args())
