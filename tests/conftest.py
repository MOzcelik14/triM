import os
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication


@pytest.fixture(scope="session", autouse=True)
def qapp():
    app = CutlineApplication.instance()
    if app is None:
        app = CutlineApplication([])
    return app
