from __future__ import annotations

import logging
import os
import sys

from cutline.app.application import CutlineApplication


def setup_logging() -> None:
    log_format = "%(asctime)s [%(levelname)s] %(name)s: %(message)s"
    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        handlers=[logging.StreamHandler(sys.stdout)],
    )


def main() -> int:
    setup_logging()
    logger = logging.getLogger("cutline")
    logger.info("Starting Cutline Video Editor...")

    # Ensure Wayland and X11 compatibility
    # If QT_QPA_PLATFORM is not set, allow Qt to auto-detect wayland/xcb
    if "QT_QPA_PLATFORM" not in os.environ and os.environ.get("WAYLAND_DISPLAY"):
        os.environ["QT_QPA_PLATFORM"] = "wayland;xcb"

    app = CutlineApplication(sys.argv)
    initial_project = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else None
    return app.start(initial_project)


if __name__ == "__main__":
    sys.exit(main())
