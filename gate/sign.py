#!/usr/bin/env python3
"""web3 wallet signatures for cyberia.my's acts: keccak-256, secp256k1 recovery, EIP-191 personal_sign.

the verifier is apparatus's `scripts/sign.py` (cyberia-to/apparatus, projects/command.md), carried here unchanged in its
arithmetic so the two houses verify the same way: stdlib only, one file on the front and in CI. it holds no secret
and verifies public signatures over public messages, so constant time is not needed. `sign` below exists for the
tests and for the local dev signer only — a real act is signed by a wallet.
"""

M64 = (1 << 64) - 1
ROT = ((0, 36, 3, 41, 18), (1, 44, 10, 45, 2), (62, 6, 43, 15, 61), (28, 55, 25, 21, 56), (27, 20, 39, 8, 14))  # ROT[x][y]


def _round_constants():
    """the 24 iota constants from the LFSR of FIPS 202 (the Keccak team's CompactFIPS202), not a typed table."""
    out, r = [], 1
    for _ in range(24):
        rc = 0
        for j in range(7):
            r = ((r << 1) ^ ((r >> 7) * 0x71)) % 256
            if r & 2: rc ^= 1 << ((1 << j) - 1)
        out.append(rc)
    return out


RC = _round_constants()


def _rol(v, n): return ((v << n) | (v >> (64 - n))) & M64 if n else v


def _permute(s):
    for rc in RC:
        c = [s[x] ^ s[x + 5] ^ s[x + 10] ^ s[x + 15] ^ s[x + 20] for x in range(5)]
        d = [c[(x - 1) % 5] ^ _rol(c[(x + 1) % 5], 1) for x in range(5)]
        s = [s[i] ^ d[i % 5] for i in range(25)]
        b = [0] * 25
        for x in range(5):
            for y in range(5):
                b[y + 5 * ((2 * x + 3 * y) % 5)] = _rol(s[x + 5 * y], ROT[x][y])
        s = [b[i] ^ (~b[(i % 5 + 1) % 5 + 5 * (i // 5)] & b[(i % 5 + 2) % 5 + 5 * (i // 5)]) for i in range(25)]
        s[0] ^= rc
    return s


def sponge(data, pad=0x01, rate=136, out=32):
    """keccak[c=512] sponge; pad 0x01 is Ethereum's keccak-256, 0x06 is FIPS sha3-256 (used only to test the permutation)."""
    msg = bytearray(data) + bytes([pad]) + bytes((-len(data) - 1) % rate)
    msg[-1] |= 0x80
    s = [0] * 25
    for off in range(0, len(msg), rate):
        for i in range(rate // 8):
            s[i] ^= int.from_bytes(msg[off + 8 * i: off + 8 * i + 8], "little")
        s = _permute(s)
    return b"".join(s[i].to_bytes(8, "little") for i in range(rate // 8))[:out]


def keccak256(data): return sponge(bytes(data))


# secp256k1
P = 2**256 - 2**32 - 977
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
G = (0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798, 0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8)


def _add(a, b):
    if a is None: return b
    if b is None: return a
    if a[0] == b[0] and (a[1] + b[1]) % P == 0: return None
    lam = (3 * a[0] * a[0] * pow(2 * a[1], -1, P)) % P if a == b else ((b[1] - a[1]) * pow(b[0] - a[0], -1, P)) % P
    x = (lam * lam - a[0] - b[0]) % P
    return (x, (lam * (a[0] - x) - a[1]) % P)


def _mul(k, pt):
    acc = None
    while k:
        if k & 1: acc = _add(acc, pt)
        pt = _add(pt, pt); k >>= 1
    return acc


def checksum(addr20):
    """EIP-55 mixed-case hex of a 20-byte address."""
    h = addr20.hex(); d = keccak256(h.encode()).hex()
    return "0x" + "".join(c.upper() if c.isalpha() and int(d[i], 16) >= 8 else c for i, c in enumerate(h))


def eip191_hash(message):
    m = message.encode() if isinstance(message, str) else bytes(message)
    return keccak256(b"\x19Ethereum Signed Message:\n" + str(len(m)).encode() + m)


def recover(message, signature):
    """the EIP-55 address that signed `message` with personal_sign, or ValueError."""
    sig = bytes.fromhex(signature[2:] if signature.startswith("0x") else signature)
    if len(sig) != 65: raise ValueError("a signature is 65 bytes")
    r, s, v = int.from_bytes(sig[:32], "big"), int.from_bytes(sig[32:64], "big"), sig[64]
    v = v - 27 if v >= 27 else v
    if v not in (0, 1) or not (0 < r < N and 0 < s < N): raise ValueError("signature out of range")
    y2 = (pow(r, 3, P) + 7) % P
    y = pow(y2, (P + 1) // 4, P)
    if y * y % P != y2: raise ValueError("r is not on the curve")
    if y & 1 != v: y = P - y
    e = int.from_bytes(eip191_hash(message), "big")
    rinv = pow(r, -1, N)
    q = _add(_mul(s * rinv % N, (r, y)), _mul((-e * rinv) % N, G))
    if q is None: raise ValueError("no key")
    return checksum(keccak256(q[0].to_bytes(32, "big") + q[1].to_bytes(32, "big"))[-20:])



def address_of(key):
    """the EIP-55 address of a private key (an int)."""
    q = _mul(key, G)
    return checksum(keccak256(q[0].to_bytes(32, "big") + q[1].to_bytes(32, "big"))[-20:])


def sign(message, key):
    """personal_sign with a private key (an int) — tests and the dev signer only. the nonce is derived from the key
    and the message, so a signature is reproducible; nothing here is constant time."""
    e = int.from_bytes(eip191_hash(message), "big")
    k = int.from_bytes(keccak256(key.to_bytes(32, "big") + e.to_bytes(32, "big")), "big") % (N - 1) + 1
    while True:
        pt = _mul(k, G)
        r = pt[0] % N
        s = pow(k, -1, N) * (e + r * key) % N
        if r and s:
            v = pt[1] & 1
            if s > N // 2:   # low-s, as wallets sign
                s, v = N - s, v ^ 1
            return "0x" + r.to_bytes(32, "big").hex() + s.to_bytes(32, "big").hex() + bytes([27 + v]).hex()
        k = k % (N - 1) + 1
