import sys
import unittest

from typing import Any
from importlib import import_module

import boytacean


def _has_module(name: str) -> bool:
    from importlib.util import find_spec

    try:
        return find_spec(name) is not None
    except (ImportError, ValueError):
        return False


def _import_video(pillow: bool = True):
    # imports a new instance of the video module, as if Pillow was not
    # installed when requested, restoring the previous modules afterwards
    names = ("boytacean.video", "PIL", "PIL.Image")
    modules = {name: sys.modules.get(name) for name in names}
    video = getattr(boytacean, "video", None)
    try:
        sys.modules.pop("boytacean.video", None)
        if not pillow:
            sys.modules["PIL"] = None
            sys.modules["PIL.Image"] = None
        return import_module("boytacean.video")
    finally:
        for name, module in modules.items():
            if module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module
        boytacean.video = video


requires_pillow = unittest.skipUnless(
    _has_module("PIL"), "Pillow not installed; skipping image-dependent test"
)


class VideoTest(unittest.TestCase):

    @requires_pillow
    def test_import(self):
        from PIL.Image import Image

        video = _import_video()
        self.assertIs(video.Image, Image)

    def test_import_without_pillow(self):
        video = _import_video(pillow=False)
        self.assertIs(video.Image, Any)

        capture = video.VideoCapture(fps=10)
        try:
            self.assertEqual(capture.formats("webm"), ["vp8", "vp9"])
            self.assertTrue(capture.should_capture(6))
            self.assertFalse(capture.should_capture(5))
        finally:
            capture.cleanup()
