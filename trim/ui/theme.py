"""triM. user interface styling and theme.

Minimalist, technical, neutral dark gray palette with signature Precision Amber accent.
"""

DARK_THEME_QSS = """
QMainWindow, QDialog {
    background-color: #151518;
    color: #e4e4ea;
}

QWidget {
    background-color: #151518;
    color: #e4e4ea;
    font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    font-size: 12px;
}

QSplitter::handle {
    background-color: #24242a;
}

QSplitter::handle:hover {
    background-color: #e07a38;
}

QToolBar {
    background-color: #1c1c20;
    border-bottom: 1px solid #28282e;
    padding: 3px 6px;
    spacing: 5px;
}

QToolButton {
    background-color: transparent;
    color: #dedee6;
    border: 1px solid transparent;
    border-radius: 3px;
    padding: 5px 9px;
    font-weight: 500;
}

QToolButton:hover {
    background-color: #282830;
    border-color: #363640;
    color: #ffffff;
}

QToolButton:pressed {
    background-color: #e07a38;
    color: #ffffff;
}

QPushButton {
    background-color: #24242b;
    color: #e4e4ea;
    border: 1px solid #33333c;
    border-radius: 3px;
    padding: 6px 14px;
    font-weight: 500;
}

SingleTrackHeader QPushButton {
    padding: 0px;
    margin: 0px;
    text-align: center;
}

QPushButton:hover {
    background-color: #2f2f38;
    border-color: #42424e;
    color: #ffffff;
}

QPushButton:pressed {
    background-color: #e07a38;
    color: #ffffff;
    border-color: #e07a38;
}

QPushButton:disabled {
    background-color: #1c1c20;
    color: #555560;
    border-color: #25252c;
}

QPushButton#PrimaryButton {
    background-color: #e07a38;
    border-color: #cb6929;
    color: #ffffff;
    font-weight: 600;
}

QPushButton#PrimaryButton:hover {
    background-color: #e88a4c;
    border-color: #d87532;
}

QPushButton#PrimaryButton:pressed {
    background-color: #c96728;
}

QTabWidget::pane {
    border: 1px solid #28282e;
    background-color: #151518;
}

QTabBar::tab {
    background-color: #1c1c20;
    color: #8e8e98;
    padding: 6px 14px;
    border-top-left-radius: 3px;
    border-top-right-radius: 3px;
    margin-right: 2px;
}

QTabBar::tab:selected {
    background-color: #24242b;
    color: #ffffff;
    border-bottom: 2px solid #e07a38;
}

QListWidget, QTreeView, QTableWidget {
    background-color: #1c1c20;
    border: 1px solid #28282e;
    border-radius: 3px;
    color: #e4e4ea;
    alternate-background-color: #202026;
}

QListWidget::item:selected, QTreeView::item:selected {
    background-color: #3b281d;
    color: #ffffff;
    border-left: 2px solid #e07a38;
}

QListWidget::item:hover, QTreeView::item:hover {
    background-color: #282830;
}

QScrollBar:vertical {
    border: none;
    background: #151518;
    width: 8px;
    margin: 0;
}

QScrollBar::handle:vertical {
    background: #2e2e38;
    min-height: 20px;
    border-radius: 3px;
}

QScrollBar::handle:vertical:hover {
    background: #424250;
}

QScrollBar:horizontal {
    border: none;
    background: #151518;
    height: 8px;
    margin: 0;
}

QScrollBar::handle:horizontal {
    background: #2e2e38;
    min-width: 20px;
    border-radius: 3px;
}

QScrollBar::handle:horizontal:hover {
    background: #424250;
}

QScrollBar::add-line, QScrollBar::sub-line {
    border: none;
    background: none;
}

QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox {
    background-color: #1f1f25;
    border: 1px solid #2e2e38;
    border-radius: 3px;
    padding: 4px 7px;
    color: #e4e4ea;
}

QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus {
    border-color: #e07a38;
}

QGroupBox {
    border: 1px solid #28282e;
    border-radius: 4px;
    margin-top: 10px;
    padding-top: 12px;
    font-weight: bold;
    color: #a4a4b2;
}

QGroupBox::title {
    subcontrol-origin: margin;
    subcontrol-position: top left;
    left: 8px;
    padding: 0 4px;
}

QProgressBar {
    background-color: #1c1c20;
    border: 1px solid #28282e;
    border-radius: 3px;
    text-align: center;
    color: #ffffff;
}

QProgressBar::chunk {
    background-color: #e07a38;
    border-radius: 2px;
}

QStatusBar {
    background-color: #121215;
    border-top: 1px solid #202026;
    color: #7a7a86;
    font-size: 11px;
}
"""
