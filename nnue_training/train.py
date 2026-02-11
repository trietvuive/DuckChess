import os
import argparse
import torch
import torch.optim as optim
from torch.utils.data import DataLoader, TensorDataset
from tqdm import tqdm

from config import BATCH_SIZE, LEARNING_RATE, WEIGHT_DECAY, NUM_EPOCHS, VALIDATION_SPLIT, GAMES_PER_EPOCH, CHECKPOINT_DIR
from model import NNUE, NNUELoss, count_parameters
from data_gen import generate_batch_data, save_data, load_data


def create_dataloaders(white_feat, black_feat, stm, evals, batch_size, val_split=0.1):
    dataset = TensorDataset(
        torch.from_numpy(white_feat),
        torch.from_numpy(black_feat),
        torch.from_numpy(stm),
        torch.from_numpy(evals) / 1000.0
    )
    val_size = int(len(dataset) * val_split)
    train_ds, val_ds = torch.utils.data.random_split(dataset, [len(dataset) - val_size, val_size])
    return (
        DataLoader(train_ds, batch_size=batch_size, shuffle=True, num_workers=0, pin_memory=True),
        DataLoader(val_ds, batch_size=batch_size, shuffle=False, num_workers=0, pin_memory=True)
    )


def train_epoch(model, loader, optimizer, criterion, device):
    model.train()
    total_loss, n = 0.0, 0
    for w, b, s, t in tqdm(loader, desc="Training", leave=False):
        w, b, s, t = w.to(device), b.to(device), s.to(device), t.to(device)
        optimizer.zero_grad()
        loss = criterion(model(w, b, s), t, model)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        optimizer.step()
        total_loss += loss.item()
        n += 1
    return total_loss / n


def validate(model, loader, device):
    model.eval()
    mse, n = 0.0, 0
    with torch.no_grad():
        for w, b, s, t in loader:
            w, b, s, t = w.to(device), b.to(device), s.to(device), t.to(device)
            mse += ((model(w, b, s) - t) ** 2).sum().item()
            n += t.size(0)
    return (mse / n) ** 0.5 * 1000


def save_checkpoint(model, optimizer, epoch, loss, filename):
    os.makedirs(CHECKPOINT_DIR, exist_ok=True)
    torch.save({
        'epoch': epoch, 'model_state_dict': model.state_dict(),
        'optimizer_state_dict': optimizer.state_dict(), 'loss': loss,
    }, os.path.join(CHECKPOINT_DIR, filename))


def main(args):
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")
    
    model = NNUE().to(device)
    print(f"Parameters: {count_parameters(model):,}")
    
    optimizer = optim.AdamW(model.parameters(), lr=args.lr, weight_decay=WEIGHT_DECAY)
    scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=args.epochs)
    criterion = NNUELoss(lambda_reg=0.0001)
    
    if args.data_file and os.path.exists(os.path.join("data", args.data_file)):
        white_feat, black_feat, stm, evals = load_data(args.data_file)
    else:
        white_feat, black_feat, stm, evals = generate_batch_data(args.num_games, args.workers)
        save_data(white_feat, black_feat, stm, evals, "training_data.npz")
    
    train_loader, val_loader = create_dataloaders(white_feat, black_feat, stm, evals, args.batch_size, VALIDATION_SPLIT)
    print(f"Train: {len(train_loader.dataset)}, Val: {len(val_loader.dataset)}")
    
    best_rmse = float('inf')
    for epoch in range(args.epochs):
        train_loss = train_epoch(model, train_loader, optimizer, criterion, device)
        val_rmse = validate(model, val_loader, device)
        scheduler.step()
        
        print(f"Epoch {epoch+1}/{args.epochs} - Loss: {train_loss:.6f}, Val RMSE: {val_rmse:.1f} cp")
        
        if val_rmse < best_rmse:
            best_rmse = val_rmse
            save_checkpoint(model, optimizer, epoch, train_loss, "best_model.pt")
    
    save_checkpoint(model, optimizer, args.epochs - 1, train_loss, "final_model.pt")
    print(f"Done! Best RMSE: {best_rmse:.1f} cp")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--epochs", type=int, default=NUM_EPOCHS)
    parser.add_argument("--batch-size", type=int, default=BATCH_SIZE)
    parser.add_argument("--lr", type=float, default=LEARNING_RATE)
    parser.add_argument("--num-games", type=int, default=GAMES_PER_EPOCH)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--data-file", type=str, default=None)
    main(parser.parse_args())
