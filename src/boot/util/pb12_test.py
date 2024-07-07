#!/usr/bin/python
# -*- coding: utf-8 -*-

import unittest

from pb12 import pb12_encode, pb12_decode


class TestPB12Compression(unittest.TestCase):
    def test_compression_decompression(self):
        compressed_data = b"".join(
            pb12_encode(b"hello world! hello world! hello world!")
        )
        decompressed_data = b"".join(pb12_decode(compressed_data))
        self.assertEqual(
            decompressed_data, b"hello world! hello world! hello world!\x00"
        )

    def test_empty_input(self):
        compressed_data = b"".join(pb12_encode(b""))
        decompressed_data = b"".join(pb12_decode(compressed_data))
        self.assertEqual(decompressed_data, b"")

    def test_single_byte_input(self):
        compressed_data = b"".join(pb12_encode(b"aaaa"))
        decompressed_data = b"".join(pb12_decode(compressed_data))
        self.assertEqual(decompressed_data, b"aaaa")

    def test_repeating_bytes(self):
        compressed_data = b"".join(pb12_encode(b"aaaaaa"))
        decompressed_data = b"".join(pb12_decode(compressed_data))
        self.assertEqual(decompressed_data, b"aaaaaa\x00\x00")

    def test_complex(self):
        compressed_data = b"".join(pb12_encode(b"\x1e\x1e\x1e>>"))
        decompressed_data = b"".join(pb12_decode(compressed_data))
        self.assertEqual(decompressed_data, b"\x1e\x1e\x1e>>\x00\x00")


if __name__ == "__main__":
    unittest.main()
