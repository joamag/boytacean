#!/usr/bin/python
# -*- coding: utf-8 -*-

import sys

def print_buffer(filename):
    file = open(filename, "rb")
    try: data = file.read()
    finally: file.close()

    buffer = [str(ord(byte)) for byte in data]
    buffer_s = ", ".join(buffer)

    print("[%s]" % buffer_s)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Missing arguments")
        exit(1)

    print_buffer(sys.argv[1])
