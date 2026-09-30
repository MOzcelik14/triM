"""Cutline user interface styling and theme."""

DARK_THEME_QSS = """
QMainWindow, QDialog {
    background-color: #1a1a1e;
    color: #e2e2e8;
}

QWidget {
    background-color: #1a1a1e;
    color: #e2e2e8;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    font-size: 12px;
}

QSplitter::handle {
    background-color: #2b2b32;
}

QSplitter::handle:hover {
    background-color: #0e86d4;
}

QToolBar {
    background-color: #202026;
    border-bottom: 1px solid #2b2b32;
    padding: 3px;
    spacing: 6px;
}

QToolButton {
    background-color: transparent;
    color: #e2e2e8;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 5px 9px;
    font-weight: 500;
}

QToolButton:hover {
    background-color: #2e2e38;
    border-color: #3e3e4a;
}

QToolButton:pressed {
    background-color: #0e86d4;
    color: #ffffff;
}

QPushButton {
    background-color: #2c2c36;
    color: #e2e2e8;
    border: 1px solid #3c3c48;
    border-radius: 4px;
    padding: 6px 14px;
    font-weight: 500;
}

QPushButton:hover {
    background-color: #383844;
    border-color: #4a4a58;
}

QPushButton:pressed {
    background-color: #0e86d4;
    color: #ffffff;
    border-color: #0e86d4;
}

QPushButton:disabled {
    background-color: #202026;
    color: #555560;
    border-color: #282830;
}

QPushButton#PrimaryButton {
    background-color: #0e86d4;
    border-color: #0c75ba;
    color: #ffffff;
}

QPushButton#PrimaryButton:hover {
    background-color: #1a96e8;
}

QTabWidget::pane {
    border: 1px solid #2b2b32;
    background-color: #1a1a1e;
}

QTabBar::tab {
    background-color: #202026;
    color: #9e9ea8;
    padding: 7px 16px;
    border-top-left-radius: 4px;
    border-top-right-radius: 4px;
    margin-right: 2px;
}

QTabBar::tab:selected {
    background-color: #282832;
    color: #ffffff;
    border-bottom: 2px solid #0e86d4;
}

QListWidget, QTreeView, QTableWidget {
    background-color: #202026;
    border: 1px solid #2b2b32;
    border-radius: 4px;
    color: #e2e2e8;
    alternate-background-color: #24242c;
}

QListWidget::item:selected, QTreeView::item:selected {
    background-color: #284464;
    color: #ffffff;
}

QListWidget::item:hover, QTreeView::item:hover {
    background-color: #2b2b36;
}

QScrollBar:vertical {
    border: none;
    background: #1a1a1e;
    width: 10px;
    margin: 0;
}

QScrollBar::handle:vertical {
    background: #363642;
    min-height: 20px;
    border-radius: 4px;
}

QScrollBar::handle:vertical:hover {
    background: #4a4a58;
}

QScrollBar:horizontal {
    border: none;
    background: #1a1a1e;
    height: 10px;
    margin: 0;
}

QScrollBar::handle:horizontal {
    background: #363642;
    min-width: 20px;
    border-radius: 4px;
}

QScrollBar::handle:horizontal:hover {
    background: #4a4a58;
}

QScrollBar::add-line, QScrollBar::sub-line {
    border: none;
    background: none;
}

QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox {
    background-color: #25252e;
    border: 1px solid #363642;
    border-radius: 4px;
    padding: 5px 8px;
    color: #e2e2e8;
}

QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus {
    border-color: #0e86d4;
}

QGroupBox {
    border: 1px solid #2b2b32;
    border-radius: 5px;
    margin-top: 10px;
    padding-top: 12px;
    font-weight: bold;
    color: #b0b0bc;
}

QGroupBox::title {
    subcontrol-origin: margin;
    subcontrol-position: top left;
    left: 8px;
    padding: 0 4px;
}

QProgressBar {
    background-color: #202026;
    border: 1px solid #2b2b32;
    border-radius: 4px;
    text-align: center;
    color: #ffffff;
}

QProgressBar::chunk {
    background-color: #0e86d4;
    border-radius: 3px;
}

QStatusBar {
    background-color: #161619;
    border-top: 1px solid #24242a;
    color: #888892;
    font-size: 11px;
}
"""
