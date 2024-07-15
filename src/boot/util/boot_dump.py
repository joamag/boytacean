#!/usr/bin/python
# -*- coding: utf-8 -*-

"""
Simple script to dump the binary contents of a file in different formats.
Useful to statically include the binary content of a file in a Rust file
to ship its contents as part of the binary.

Usage:
    python boot_dump.py <filename> [mode]

    filename: the file to dump
    mode: the output format (buffer, hex, hexbytes, bytes)
"""

import sys
import binascii


def print_buffer(filename, mode="buffer"):
    file = open(filename, "rb")
    try:
        data = file.read()
    finally:
        file.close()

    if mode == "buffer":
        buffer = [str(_ord(byte)) for byte in data]
        buffer_s = ", ".join(buffer)
        print("[%s]" % buffer_s)

    elif mode == "hex":
        counter = 0
        for byte in data:
            print("[0x%04x] 0x%02x" % (counter, _ord(byte)))
            counter += 1

    elif mode == "hexbytes":
        hex_data = binascii.hexlify(data).decode()
        escaped_data = "".join(
            "\\x" + hex_data[i : i + 2] for i in range(0, len(hex_data), 2)
        )
        print("b'%s'" % escaped_data)

    elif mode == "bytes":
        print(repr(data))


def _ord(byte):
    return byte if isinstance(byte, int) else ord(byte)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Missing arguments")
        exit(1)

    mode = "buffer"

    if len(sys.argv) > 2:
        mode = sys.argv[2]

    print_buffer(sys.argv[1], mode=mode)
