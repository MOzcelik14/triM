#!/usr/bin/env python3
"""Generates triM. brand assets: SVG wordmark, SVG icon, and multi-resolution PNG icons."""

import os
from pathlib import Path

# Ensure offscreen Qt
os.environ["QT_QPA_PLATFORM"] = "offscreen"
from PySide6.QtCore import QByteArray, QSize, Qt
from PySide6.QtGui import QColor, QGuiApplication, QImage, QPainter
from PySide6.QtSvg import QSvgRenderer

app = QGuiApplication.instance() or QGuiApplication([])

BASE_DIR = Path(__file__).resolve().parent.parent
BRANDING_DIR = BASE_DIR / "trim" / "resources" / "branding"
ICONS_DIR = BASE_DIR / "trim" / "resources" / "icons"

BRANDING_DIR.mkdir(parents=True, exist_ok=True)
ICONS_DIR.mkdir(parents=True, exist_ok=True)

# 1. Pure Typographic Wordmark SVG: triM.
WORDMARK_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 120" width="400" height="120">
  <style>
    .wordmark {
      font-family: -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
      font-size: 84px;
      letter-spacing: -2px;
    }
    .tri {
      fill: #EDEDF2;
      font-weight: 550;
    }
    .m-char {
      fill: #FFFFFF;
      font-weight: 800;
    }
    .dot {
      fill: #E07A38;
      font-weight: 900;
    }
  </style>
  <text x="30" y="88" class="wordmark">
    <tspan class="tri">tri</tspan><tspan class="m-char">M</tspan><tspan class="dot">.</tspan>
  </text>
</svg>
"""

# 2. Geometric Application Icon SVG: io.github.mozcelik14.triM.svg
# Represents timeline clips meeting at a precision cut/trim marker
APP_ICON_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <defs>
    <!-- Background rounded rectangle gradient -->
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#1B1B1F" />
      <stop offset="100%" stop-color="#121215" />
    </linearGradient>
    <!-- Left clip subtle gradient -->
    <linearGradient id="leftClipGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#2D2D36" />
      <stop offset="100%" stop-color="#343440" />
    </linearGradient>
    <!-- Right clip subtle gradient -->
    <linearGradient id="rightClipGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#23232B" />
      <stop offset="100%" stop-color="#282832" />
    </linearGradient>
    <!-- Playhead / Trim line glow -->
    <filter id="cutGlow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="0" stdDeviation="4" flood-color="#E07A38" flood-opacity="0.3" />
    </filter>
  </defs>

  <!-- Base container: Squircle -->
  <rect x="24" y="24" width="464" height="464" rx="108" fill="url(#bgGrad)" stroke="#2B2B33" stroke-width="3" />

  <!-- Timeline track container -->
  <rect x="68" y="160" width="376" height="192" rx="14" fill="#18181D" stroke="#26262E" stroke-width="2" />

  <!-- Left Clip (Active trimmed segment) -->
  <rect x="76" y="172" width="168" height="168" rx="8" fill="url(#leftClipGrad)" />
  <!-- Left clip internal frame notches -->
  <line x1="132" y1="172" x2="132" y2="340" stroke="#3A3A48" stroke-width="1.5" stroke-dasharray="4,6" opacity="0.4" />
  <line x1="188" y1="172" x2="188" y2="340" stroke="#3A3A48" stroke-width="1.5" stroke-dasharray="4,6" opacity="0.4" />

  <!-- Right Clip (Adjacent cut segment) -->
  <rect x="268" y="172" width="168" height="168" rx="8" fill="url(#rightClipGrad)" />
  <line x1="324" y1="172" x2="324" y2="340" stroke="#2E2E3A" stroke-width="1.5" stroke-dasharray="4,6" opacity="0.4" />
  <line x1="380" y1="172" x2="380" y2="340" stroke="#2E2E3A" stroke-width="1.5" stroke-dasharray="4,6" opacity="0.4" />

  <!-- Precision Trim Cut Marker & Playhead -->
  <!-- Top Marker Head -->
  <polygon points="256,128 244,148 268,148" fill="#E07A38" />

  <!-- Playhead Vertical Line -->
  <line x1="256" y1="148" x2="256" y2="364" stroke="#E07A38" stroke-width="12" stroke-linecap="round" filter="url(#cutGlow)" />

  <!-- Bottom Marker Anchor -->
  <polygon points="256,384 244,364 268,364" fill="#E07A38" />

  <!-- Trim Brackets: Opposing precision brackets at cut point -->
  <!-- Left In-Trim Bracket [ -->
  <path d="M 238,228 L 226,228 L 226,284 L 238,284" fill="none" stroke="#E07A38" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" opacity="0.85" />

  <!-- Right Out-Trim Bracket ] -->
  <path d="M 274,228 L 286,228 L 286,284 L 274,284" fill="none" stroke="#E07A38" stroke-width="6" stroke-linecap="round" stroke-linejoin="round" opacity="0.85" />
</svg>
"""

# Save SVGs
wordmark_path = BRANDING_DIR / "logo_wordmark.svg"
wordmark_path.write_text(WORDMARK_SVG, encoding="utf-8")

app_icon_svg_path = BRANDING_DIR / "app_icon.svg"
app_icon_svg_path.write_text(APP_ICON_SVG, encoding="utf-8")

# Also save FreeDesktop app ID named SVG
desktop_icon_svg_path = BASE_DIR / "io.github.mozcelik14.triM.svg"
desktop_icon_svg_path.write_text(APP_ICON_SVG, encoding="utf-8")

print(f"Generated SVGs: {wordmark_path}, {app_icon_svg_path}, {desktop_icon_svg_path}")

# Render PNG icons of various resolutions: 16, 32, 48, 64, 128, 256, 512
renderer = QSvgRenderer(QByteArray(APP_ICON_SVG.encode("utf-8")))

sizes = [16, 32, 48, 64, 128, 256, 512]
for sz in sizes:
    img = QImage(sz, sz, QImage.Format.Format_ARGB32_Premultiplied)
    img.fill(Qt.GlobalColor.transparent)
    painter = QPainter(img)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing, True)
    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform, True)
    renderer.render(painter)
    painter.end()

    png_path = ICONS_DIR / f"trim_{sz}x{sz}.png"
    img.save(str(png_path), "PNG")
    print(f"Rendered: {png_path}")

# Main icon file
main_png = ICONS_DIR / "trim.png"
img_512 = QImage(512, 512, QImage.Format.Format_ARGB32_Premultiplied)
img_512.fill(Qt.GlobalColor.transparent)
painter = QPainter(img_512)
renderer.render(painter)
painter.end()
img_512.save(str(main_png), "PNG")
img_512.save(str(BASE_DIR / "io.github.mozcelik14.triM.png"), "PNG")

# Render wordmark PNG for about dialog / splash
wm_renderer = QSvgRenderer(QByteArray(WORDMARK_SVG.encode("utf-8")))
wm_img = QImage(400, 120, QImage.Format.Format_ARGB32_Premultiplied)
wm_img.fill(Qt.GlobalColor.transparent)
p_wm = QPainter(wm_img)
p_wm.setRenderHint(QPainter.RenderHint.Antialiasing, True)
wm_renderer.render(p_wm)
p_wm.end()
wm_png = BRANDING_DIR / "logo_wordmark.png"
wm_img.save(str(wm_png), "PNG")
print(f"Rendered Wordmark: {wm_png}")
