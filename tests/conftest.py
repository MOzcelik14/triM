import os
import pytest

os.environ["QT_QPA_PLATFORM"] = "offscreen"

from trim.app.application import CutlineApplication


@pytest.fixture(scope="session", autouse=True)
def qapp():
    from trim.core.autosave import AutosaveManager
    # Never block headless test runs with modal recovery dialogs
    AutosaveManager.has_recovery_file = lambda self: False

    app = CutlineApplication.instance()
    if app is None:
        app = CutlineApplication([])
    return app
