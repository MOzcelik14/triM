from __future__ import annotations

import logging
import os
import sys

from trim.app.application import TrimApplication


def setup_logging() -> None:
    log_format = "%(asctime)s [%(levelname)s] %(name)s: %(message)s"
    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        handlers=[logging.StreamHandler(sys.stdout)],
    )


def main() -> int:
    setup_logging()
    logger = logging.getLogger("trim")
    logger.info("Starting triM. Video Editor...")

    # Ensure Wayland and X11 compatibility
    if "QT_QPA_PLATFORM" not in os.environ and os.environ.get("WAYLAND_DISPLAY"):
        os.environ["QT_QPA_PLATFORM"] = "wayland;xcb"

    app = TrimApplication(sys.argv)
    initial_project = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("-") else None
    return app.start(initial_project)


if __name__ == "__main__":
    sys.exit(main())
