import torch
import torch.nn as nn
from config import INPUT_SIZE, HIDDEN1_SIZE, HIDDEN2_SIZE, OUTPUT_SIZE


class ClippedReLU(nn.Module):
    def forward(self, x):
        return torch.clamp(x, 0.0, 1.0)


class NNUE(nn.Module):
    def __init__(self):
        super().__init__()
        self.input_layer = nn.Linear(INPUT_SIZE, HIDDEN1_SIZE)
        self.hidden1 = nn.Linear(HIDDEN1_SIZE * 2, HIDDEN2_SIZE)
        self.output = nn.Linear(HIDDEN2_SIZE, OUTPUT_SIZE)
        self.clipped_relu = ClippedReLU()
        self._init_weights()
    
    def _init_weights(self):
        for module in self.modules():
            if isinstance(module, nn.Linear):
                nn.init.kaiming_normal_(module.weight, nonlinearity='relu')
                if module.bias is not None:
                    nn.init.zeros_(module.bias)
    
    def forward(self, white_features, black_features, stm_white):
        white_hidden = self.clipped_relu(self.input_layer(white_features))
        black_hidden = self.clipped_relu(self.input_layer(black_features))
        
        us = stm_white * white_hidden + (1 - stm_white) * black_hidden
        them = stm_white * black_hidden + (1 - stm_white) * white_hidden
        combined = torch.cat([us, them], dim=1)
        
        x = self.clipped_relu(self.hidden1(combined))
        return self.output(x)


class NNUELoss(nn.Module):
    def __init__(self, lambda_reg=0.01):
        super().__init__()
        self.lambda_reg = lambda_reg
        self.mse = nn.MSELoss()
    
    def forward(self, pred, target, model):
        mse_loss = self.mse(pred, target)
        l2_reg = sum(p.pow(2).sum() for p in model.parameters())
        return mse_loss + self.lambda_reg * l2_reg


def count_parameters(model):
    return sum(p.numel() for p in model.parameters() if p.requires_grad)


if __name__ == "__main__":
    model = NNUE()
    print(f"NNUE Model: {count_parameters(model):,} parameters")
