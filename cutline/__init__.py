"""Backwards compatibility shim for cutline -> trim."""
from __future__ import annotations

import sys
import trim
import trim.app
import trim.app.application
import trim.commands
import trim.commands.timeline_commands
import trim.core
import trim.core.autosave
import trim.core.clip
import trim.core.media
import trim.core.project
import trim.core.timeline
import trim.core.track
import trim.export
import trim.export.exporter
import trim.export.presets
import trim.main
import trim.media
import trim.media.ffprobe
import trim.media.playback
import trim.media.thumbnails
import trim.media.waveform
import trim.ui
import trim.ui.dialogs
import trim.ui.dialogs.export_dialog
import trim.ui.dialogs.title_dialog
import trim.ui.inspector
import trim.ui.inspector.inspector_widget
import trim.ui.main_window
import trim.ui.preview
import trim.ui.preview.monitor_widget
import trim.ui.preview.transport_bar
import trim.ui.preview.vu_meter
import trim.ui.project_bin
import trim.ui.project_bin.media_bin_widget
import trim.ui.theme
import trim.ui.timeline
import trim.ui.timeline.time_ruler
import trim.ui.timeline.timeline_canvas
import trim.ui.timeline.timeline_widget
import trim.ui.timeline.track_header

# Register all modules in sys.modules under cutline
_modules_map = {
    "cutline": trim,
    "cutline.app": trim.app,
    "cutline.app.application": trim.app.application,
    "cutline.commands": trim.commands,
    "cutline.commands.timeline_commands": trim.commands.timeline_commands,
    "cutline.core": trim.core,
    "cutline.core.autosave": trim.core.autosave,
    "cutline.core.clip": trim.core.clip,
    "cutline.core.media": trim.core.media,
    "cutline.core.project": trim.core.project,
    "cutline.core.timeline": trim.core.timeline,
    "cutline.core.track": trim.core.track,
    "cutline.export": trim.export,
    "cutline.export.exporter": trim.export.exporter,
    "cutline.export.presets": trim.export.presets,
    "cutline.main": trim.main,
    "cutline.media": trim.media,
    "cutline.media.ffprobe": trim.media.ffprobe,
    "cutline.media.playback": trim.media.playback,
    "cutline.media.thumbnails": trim.media.thumbnails,
    "cutline.media.waveform": trim.media.waveform,
    "cutline.ui": trim.ui,
    "cutline.ui.dialogs": trim.ui.dialogs,
    "cutline.ui.dialogs.export_dialog": trim.ui.dialogs.export_dialog,
    "cutline.ui.dialogs.title_dialog": trim.ui.dialogs.title_dialog,
    "cutline.ui.inspector": trim.ui.inspector,
    "cutline.ui.inspector.inspector_widget": trim.ui.inspector.inspector_widget,
    "cutline.ui.main_window": trim.ui.main_window,
    "cutline.ui.preview": trim.ui.preview,
    "cutline.ui.preview.monitor_widget": trim.ui.preview.monitor_widget,
    "cutline.ui.preview.transport_bar": trim.ui.preview.transport_bar,
    "cutline.ui.preview.vu_meter": trim.ui.preview.vu_meter,
    "cutline.ui.project_bin": trim.ui.project_bin,
    "cutline.ui.project_bin.media_bin_widget": trim.ui.project_bin.media_bin_widget,
    "cutline.ui.theme": trim.ui.theme,
    "cutline.ui.timeline": trim.ui.timeline,
    "cutline.ui.timeline.time_ruler": trim.ui.timeline.time_ruler,
    "cutline.ui.timeline.timeline_canvas": trim.ui.timeline.timeline_canvas,
    "cutline.ui.timeline.timeline_widget": trim.ui.timeline.timeline_widget,
    "cutline.ui.timeline.track_header": trim.ui.timeline.track_header,
}

for mod_name, mod_obj in _modules_map.items():
    sys.modules[mod_name] = mod_obj
