"""float64 control implementation of classic Lenia (spec §5 continuous definition).

Not normative. Used for: comparing the integer LUT/kernel with float values,
culturing initial states, and the G2 behaviour comparison.
"""
import numpy as np


def kernel_f64(R, beta):
    B = len(beta)
    side = 2 * R + 1
    dy, dx = np.mgrid[-R:R + 1, -R:R + 1]
    r = np.sqrt(dx * dx + dy * dy) / R
    K = np.zeros((side, side))
    m = r < 1
    br = B * r[m]
    i = np.floor(br).astype(int)
    s = br - i
    with np.errstate(divide="ignore", over="ignore", invalid="ignore"):
        core = np.where((s > 0) & (s < 1), np.exp(4 - 1 / (s * (1 - s))), 0.0)
    b = np.array([p / q for p, q in beta])
    K[m] = b[i] * core
    return K / K.sum()


def growth(u, mu, sigma):
    return 2 * np.exp(-((u - mu) ** 2) / (2 * sigma ** 2)) - 1


class Lenia:
    def __init__(self, N, R, T, mu, sigma, beta):
        self.N, self.T, self.mu, self.sigma = N, T, mu, sigma
        K = kernel_f64(R, beta)
        pad = np.zeros((N, N))
        side = 2 * R + 1
        pad[:side, :side] = K
        pad = np.roll(pad, (-R, -R), axis=(0, 1))
        # correlation with a symmetric kernel equals convolution
        self.fK = np.fft.fft2(pad)

    def step(self, A):
        U = np.real(np.fft.ifft2(np.fft.fft2(A) * self.fK))
        return np.clip(A + growth(U, self.mu, self.sigma) / self.T, 0, 1)


SPEC_ID = b"lenia-q16/0.2-draft"


def state_record(A_u16):
    N = A_u16.shape[0]
    hdr = b"LQ16STAT" + SPEC_ID.ljust(32, b"\0") + np.uint32(N).tobytes() + np.uint32(N).tobytes()
    return hdr + A_u16.astype("<u2").tobytes()


def read_state(path):
    b = open(path, "rb").read()
    N = int(np.frombuffer(b[40:44], "<u4")[0])
    return np.frombuffer(b[48:], "<u2").reshape(N, N)


def quantize(A):
    """float state in [0,1] -> u16 with round-half-even (numpy rint)."""
    return np.rint(np.clip(A, 0, 1) * 65535).astype(np.uint16)
