#!/usr/bin/env python3
"""Convert an nRF52840 .uf2 image to Intel HEX for a serial-DFU package.

`just flash-serial` feeds the HEX to `adafruit-nrfutil dfu genpkg`, which turns
it into the .bin inside the DFU .zip. The Adafruit nRF52 bootloader writes that
.bin at the start of the application region — 0x1000 when no SoftDevice is
present, as on an RMK board — so the image must start exactly there and be
contiguous. Anything else is rejected instead of being flashed at a wrong offset.

Usage: uf2_to_hex.py <in.uf2> <out.hex>
"""

import struct
import sys

UF2_MAGIC_START0 = 0x0A324655
UF2_MAGIC_START1 = 0x9E5D5157
UF2_MAGIC_END = 0x0AB16F30
UF2_FLAG_NOT_MAIN_FLASH = 0x00000001
UF2_FLAG_FAMILY_ID = 0x00002000
NRF52840_FAMILY = 0xADA52840
BLOCK_SIZE = 512
APP_START = 0x1000


def read_uf2(path: str) -> dict[int, bytes]:
    """Return {target address: payload} for every main-flash block in the UF2."""
    with open(path, "rb") as f:
        data = f.read()
    if not data or len(data) % BLOCK_SIZE:
        raise ValueError(f"{path}: size {len(data)} is not a multiple of {BLOCK_SIZE}")
    chunks: dict[int, bytes] = {}
    for off in range(0, len(data), BLOCK_SIZE):
        block = data[off : off + BLOCK_SIZE]
        m0, m1, flags, addr, size, _seq, _total, family = struct.unpack_from("<8I", block)
        (m_end,) = struct.unpack_from("<I", block, BLOCK_SIZE - 4)
        if (m0, m1, m_end) != (UF2_MAGIC_START0, UF2_MAGIC_START1, UF2_MAGIC_END):
            raise ValueError(f"{path}: bad UF2 magic in block at byte {off}")
        if flags & UF2_FLAG_NOT_MAIN_FLASH:
            continue
        if flags & UF2_FLAG_FAMILY_ID and family != NRF52840_FAMILY:
            raise ValueError(f"{path}: family 0x{family:08X} is not nRF52840")
        if size > 476:
            raise ValueError(f"{path}: block at byte {off} claims {size} payload bytes")
        if addr in chunks:
            raise ValueError(f"{path}: duplicate block for address 0x{addr:X}")
        chunks[addr] = block[32 : 32 + size]
    if not chunks:
        raise ValueError(f"{path}: no main-flash blocks")
    return chunks


def to_image(chunks: dict[int, bytes]) -> tuple[int, bytes]:
    """Join chunks into one contiguous image; return (start address, bytes)."""
    start = min(chunks)
    image = bytearray()
    for addr in sorted(chunks):
        expected = start + len(image)
        if addr != expected:
            raise ValueError(f"gap or overlap at 0x{addr:X} (expected 0x{expected:X})")
        image += chunks[addr]
    return start, bytes(image)


def hex_record(rtype: int, addr16: int, payload: bytes) -> str:
    body = bytes([len(payload), addr16 >> 8, addr16 & 0xFF, rtype]) + payload
    checksum = (-sum(body)) & 0xFF
    return ":" + (body + bytes([checksum])).hex().upper()


def to_intel_hex(start: int, image: bytes) -> str:
    lines = []
    upper = None
    for i in range(0, len(image), 16):
        addr = start + i
        if addr >> 16 != upper:
            upper = addr >> 16
            lines.append(hex_record(0x04, 0, struct.pack(">H", upper)))
        # A 16-byte row never crosses a 64 KiB boundary: start is 16-aligned.
        lines.append(hex_record(0x00, addr & 0xFFFF, image[i : i + 16]))
    lines.append(hex_record(0x01, 0, b""))
    return "\n".join(lines) + "\n"


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    src, dst = sys.argv[1], sys.argv[2]
    try:
        start, image = to_image(read_uf2(src))
    except (OSError, ValueError, struct.error) as exc:
        print(f"uf2_to_hex: {exc}", file=sys.stderr)
        return 1
    if start != APP_START:
        print(
            f"uf2_to_hex: {src} starts at 0x{start:X}, not 0x{APP_START:X}; "
            "serial DFU would write it at the wrong address",
            file=sys.stderr,
        )
        return 1
    with open(dst, "w", encoding="ascii") as f:
        f.write(to_intel_hex(start, image))
    print(f"uf2_to_hex: {src} -> {dst} (0x{start:X}..0x{start + len(image):X}, {len(image)} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
