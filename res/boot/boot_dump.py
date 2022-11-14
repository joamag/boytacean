#!/usr/bin/python
# -*- coding: utf-8 -*-

import sys

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Missing arguments")
        exit(1)
    file = open(sys.argv[1], "rb")
    try: data = file.read()
    finally: file.close()
    buffer = [str(byte) for byte in data]
    buffer_s = ", ".join(buffer)
    print("[" + buffer_s + "]")
