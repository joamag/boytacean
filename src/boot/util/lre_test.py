#!/usr/bin/python
# -*- coding: utf-8 -*-

from unittest import TestCase, main

from lre import lre_encode, lre_decode


class TestLRECompression(TestCase):
    def test_compression(self):
        compressed_data = lre_encode(b"\x00\x00\x00\xff\x01")
        self.assertEqual(compressed_data, b"\x03\x00\x01\xff\x01\x01\x00")

    def test_compression_decompression(self):
        compressed_data = lre_encode(b"\x00\x00\x00\xff\x01")
        decompressed_data = lre_decode(compressed_data)
        self.assertEqual(decompressed_data, b"\x00\x00\x00\xff\x01")

    def test_empty_input(self):
        compressed_data = lre_encode(b"")
        decompressed_data = lre_decode(compressed_data)
        self.assertEqual(compressed_data, b"\x00")
        self.assertEqual(decompressed_data, b"")

    def test_single_byte_input(self):
        compressed_data = lre_encode(b"a")
        decompressed_data = lre_decode(compressed_data)
        self.assertEqual(compressed_data, b"\x01a\x00")
        self.assertEqual(decompressed_data, b"a")

    def test_repeating_bytes(self):
        compressed_data = lre_encode(b"a" * 300)
        decompressed_data = lre_decode(compressed_data)
        self.assertEqual(compressed_data, b"\xffa\x2da\x00")
        self.assertEqual(decompressed_data, b"a" * 300)

    def test_complex(self):
        compressed_data = lre_encode(b"hello world! hello world! hello world!")
        decompressed_data = lre_decode(compressed_data)
        self.assertEqual(decompressed_data, b"hello world! hello world! hello world!")

    def test_end_marker(self):
        decompressed_data = lre_decode(b"\x02a\x00\x03b")
        self.assertEqual(decompressed_data, b"aa")

    def test_no_end_marker(self):
        decompressed_data = lre_decode(b"\x02a\x03b")
        self.assertEqual(decompressed_data, b"aabbb")


if __name__ == "__main__":
    main()
