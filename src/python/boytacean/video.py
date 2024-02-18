from os import remove
from glob import glob
from math import ceil
from shutil import move, rmtree
from typing import Any, Sequence, Union
from tempfile import mkdtemp

from os.path import exists, join

from PIL.Image import Image

from .boytacean import (
    DISPLAY_WIDTH,
    DISPLAY_HEIGHT,
    VISUAL_FREQ,
)

FORMATS = {
    "mp4": ["avc1", "hev1"],
    "webm": ["vp8", "vp9"],
    "mkv": ["avc1", "h264", "hev1"],
}


class VideoCapture:
    _start_frame: Union[int, None] = None
    _next_frame: Union[int, None] = None
    _video_format: str = "avc1"
    _video_extension: str = "mp4"
    _video_name: str = "output"
    _frame_gap: int = 60
    _frame_format: str = "png"
    _frame_prefix: str = "frame_"
    _capture_temp_dir: Union[str, None] = None

    def __init__(
        self,
        start_frame=0,
        video_format="avc1",
        video_extension="mp4",
        video_name="output",
        fps=5,
        frame_format="png",
    ):
        super().__init__()
        self._start_frame = start_frame
        self._video_format = video_format
        self._video_extension = video_extension
        self._video_name = video_name
        self._frame_format = frame_format
        self._frame_gap = ceil(VISUAL_FREQ / fps)
        self._next_frame = start_frame + self._frame_gap
        self._capture_temp_dir = mkdtemp()

    @classmethod
    def formats(cls, extension="mp4") -> Sequence[str]:
        return FORMATS.get(extension, [])

    def should_capture(self, frame_index) -> bool:
        return self._start_frame is None or frame_index >= self._next_frame

    def compute_next(self, frame_index):
        self._next_frame = frame_index + self._frame_gap

    def frame_path(self, frame_index) -> str:
        return f"{self._capture_temp_dir}/{self._frame_prefix}{frame_index:08d}.{self.frame_format_l}"

    def cleanup(self):
        if self._capture_temp_dir:
            rmtree(self._capture_temp_dir)

    def save_frame(self, frame: Image, frame_index: int):
        frame.save(self.frame_path(frame_index), format=self.frame_format)

    def build(self, save=False) -> Any:
        from cv2 import VideoWriter, VideoWriter_fourcc, imread
        from IPython.display import Video, FileLink

        if self._capture_temp_dir is None:
            raise RuntimeError("Not capturing a video")

        image_paths = glob(f"{self.frames_glob}")
        video_path = f"{self._capture_temp_dir}/{self.video_filename}"

        encoder = VideoWriter(
            video_path,
            VideoWriter_fourcc(*self.video_format_fourcc),
            self.fps,
            (DISPLAY_WIDTH, DISPLAY_HEIGHT),
        )

        try:
            for image_file in sorted(image_paths):
                image = imread(image_file)
                encoder.write(image)
        finally:
            encoder.release()

        if save:
            if exists(self.video_filename):
                remove(self.video_filename)
            move(video_path, ".")
            video_path = join(".", self.video_filename)

        return Video(video_path, embed=True, html_attributes="controls loop autoplay")

    @property
    def fps(self) -> float:
        return VISUAL_FREQ / self._frame_gap

    @property
    def video_filename(self) -> str:
        return f"{self._video_name}.{self._video_extension}"

    @property
    def video_format_l(self) -> str:
        return self._video_format.lower()

    @property
    def video_format_fourcc(self) -> str:
        return self.video_format_l.ljust(4, "0")

    @property
    def frame_format(self) -> str:
        return self._frame_format

    @property
    def frame_format_l(self) -> str:
        return self._frame_format.lower()

    @property
    def frames_glob(self) -> str:
        return f"{self._capture_temp_dir}/{self._frame_prefix}*.{self.frame_format_l}"
