"""Suaviza apenas os contornos: cada conjunto de nível {alfa >= t} é suavizado e re-limiarizado.
Regiões de tom plano e degradês internos permanecem idênticos; só as bordas (escadas) mudam."""
import sys, numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, zoom
src, out, sigma = sys.argv[1], sys.argv[2], float(sys.argv[3])
a = np.asarray(Image.open(src).convert("RGBA"))[..., 3].astype(np.float64) / 255.0
S = 8
up = zoom(a, S, order=1)                      # bilinear 8x
levels = np.linspace(0, 1, 129)[1:]           # 128 níveis
res = np.zeros_like(up)
for t in levels:
    m = gaussian_filter((up >= t).astype(np.float64), sigma)
    # limiar suave (antialias ~1 px)
    res += np.clip((m - 0.5) * 6 + 0.5, 0, 1)
res /= len(levels)
rgba = np.zeros(res.shape + (4,), np.uint8)
rgba[..., 3] = np.round(res * 255).astype(np.uint8)
Image.fromarray(rgba, "RGBA").save(out)
# métricas: quanto o interior mudou (pixels longe de bordas)
from scipy.ndimage import sobel
edge = np.hypot(sobel(up, 0), sobel(up, 1)) > 0.02
far = ~gaussian_filter(edge.astype(float), 3).astype(bool)
print(f"sigma={sigma} diff_interior_max={np.abs(res-up)[far].max():.4f} diff_global_mean={np.abs(res-up).mean():.4f}")
