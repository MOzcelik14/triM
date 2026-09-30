import os
from pathlib import Path
import subprocess
import tempfile
import numpy as np
import pytest

from trim.media.waveform import WaveformGenerator


def test_waveform_extraction_and_caching():
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        snd_file = tmp_path / "tone.wav"
        cache_dir = tmp_path / "cache"

        # Create 1 second tone
        subprocess.run([
            "ffmpeg", "-y", "-v", "quiet",
            "-f", "lavfi", "-i", "sine=frequency=440:duration=1.0",
            str(snd_file),
        ], check=True)

        generator = WaveformGenerator(cache_dir=cache_dir, points_per_second=100)
        peaks = generator.get_waveform(str(snd_file))

        assert peaks is not None
        assert isinstance(peaks, np.ndarray)
        assert peaks.shape[1] == 2
        # Approx 100 points for 1 second
        assert 95 <= len(peaks) <= 105
        # Sine wave peaks should be roughly between -0.13 and +0.13
        assert peaks[:, 1].max() > 0.1
        assert peaks[:, 0].min() < -0.1

        # Verify caching (calling second time loads from disk)
        cached_peaks = generator.get_waveform(str(snd_file))
        assert cached_peaks is not None
        np.testing.assert_array_equal(peaks, cached_peaks)
