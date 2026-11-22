from sys import modules
from unittest import TestCase, skipUnless

from typing import Any
from importlib import import_module


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
    previous = {name: modules.get(name) for name in names}
    package = import_module("boytacean")
    video = getattr(package, "video", None)
    try:
        modules.pop("boytacean.video", None)
        if not pillow:
            modules["PIL"] = None
            modules["PIL.Image"] = None
        return import_module("boytacean.video")
    finally:
        for name, module in previous.items():
            if module is None:
                modules.pop(name, None)
            else:
                modules[name] = module
        package.video = video


requires_pillow = skipUnless(
    _has_module("PIL"), "Pillow not installed; skipping image-dependent test"
)


class VideoTest(TestCase):

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
