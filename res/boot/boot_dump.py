#!/usr/bin/python
# -*- coding: utf-8 -*-

import sys


def print_buffer(filename, mode="buffer"):
    file = open(filename, "rb")
    try:
        data = file.read()
    finally:
        file.close()

    if mode == "buffer":
        buffer = [str(ord(byte)) for byte in data]
        buffer_s = ", ".join(buffer)

        print("[%s]" % buffer_s)

    elif mode == "hex":
        counter = 0
        for byte in data:
            print("[0x%04x] 0x%02x" % (counter, ord(byte)))
            counter += 1


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Missing arguments")
        exit(1)

    mode = "buffer"

    if len(sys.argv) > 2:
        mode = sys.argv[2]

    print_buffer(sys.argv[1], mode=mode)
