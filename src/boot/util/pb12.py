#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
This module provides functions to compress data using a custom compression algorithm
and to explain the compression process in a human-readable format.

Compression Algorithm Details:
    The pb12 algorithm processes input data byte by byte, utilizing control bits to determine
    how each byte should be handled. The control bits can represent the following:
        - 0b00: The byte is a literal and should be stored as-is.
        - 0b10: The byte is a repetition of one of the last two bytes.
            - If the byte is the same as the last byte, it is represented as 0b11.
            - If the byte is the same as the second last byte, it is represented as 0b10.
        - 0b01: The byte is a modified version of the previous byte.
            - Additional bits indicate the specific modification.

    The function maintains a list of the last two bytes processed (`prev`) to help determine
    whether a byte is a repetition or can be modified from a previous byte.

Coding Example:
    data = [97, 98, 99, 97, 98, 98, 99]
    compressed_data = pb12(data)
    for chunk in compressed_data:
        print(chunk)

Usage:
    python pb12.py <mode> <infile> <outfile>"

    mode: the mode to be used (encoding, decoding)
    infile: input file for the operation
    outfile: output file for the operation
"""

import sys

STRIP_EXTRA = False


def opts(byte):
    # top bit: 0 = left, 1 = right
    # bottom bit: 0 = or, 1 = and
    if byte is None:
        return []
    return [
        byte | (byte << 1) & 0xFF,
        byte & (byte << 1),
        byte | (byte >> 1) & 0xFF,
        byte & (byte >> 1),
    ]


def pb12_encode(data):
    data = iter(data)

    literals = bytearray()
    bits = 0
    control = 0
    prev = [None, None]

    while True:
        try:
            byte = next(data)
        except StopIteration:
            if bits == 0:
                break
            byte = 0

        if byte in prev:
            bits += 2
            control <<= 1
            control |= 1
            control <<= 1
            if byte == prev[1]:
                control |= 1  # 0b10 = out[-2], 0b11 = out[-1]
        else:
            bits += 2
            control <<= 2
            options = opts(prev[1])
            if byte in options:
                # 0b01 = modify
                control |= 1

                bits += 2
                control <<= 2
                control |= options.index(byte)
            else:
                # 0b00 = literal
                literals.append(byte)
        prev = [prev[1], byte]

        # flush control byte and literals, yielding
        # these values to the output
        if bits >= 8:
            outctl = control >> (bits - 8)
            assert outctl != 1  # that's the end byte
            yield bytes([outctl]) + literals
            bits -= 8
            control &= (1 << bits) - 1
            literals = bytearray()

    yield b"\x01"


def pb12_decode(data):
    data = iter(data)
    output = bytearray()
    prev = [None, None]

    while True:
        try:
            control = next(data)
            if control == 0x01:
                break  # End of data
        except StopIteration:
            break

        control_bits = 8
        while control_bits > 0:
            if control_bits < 2:
                break
            control_bits -= 2
            ctl = (control >> control_bits) & 0b11

            if ctl == 0b10:
                output.append(prev[0])
            elif ctl == 0b11:
                output.append(prev[1])
            elif ctl == 0b01:
                # in case there's not enough control bits left
                # then we must read a new control byte
                if control_bits < 2:
                    control = next(data)
                    control_bits = 8

                control_bits -= 2
                modify_ctl = (control >> control_bits) & 0b11
                options = opts(prev[1])
                output.append(options[modify_ctl])
            else:  # 0b00
                try:
                    literal = next(data)
                except StopIteration:
                    break
                output.append(literal)

            prev = [prev[1], output[-1]]

    yield bytes(output)


if __name__ == "__main__":
    if not len(sys.argv) > 3:
        print("Usage: pb12.py <mode> <infile> <outfile>")
        sys.exit(1)
    mode, infile, outfile = sys.argv[1:]

    if mode in ("encode", "compress"):
        operation = pb12_encode
    elif mode in ("decode", "decompress"):
        operation = pb12_decode
    else:
        raise ValueError("Invalid mode")

    with open(infile, "rb") as file:
        data = file.read()
        if STRIP_EXTRA and mode in ("encode", "compress"):
            data = data.rstrip(b"\x00")

    with open(outfile, "wb") as file:
        file.writelines(operation(data))
