#!/usr/bin/env python3
# Purpose: save the whole X display (the app running under Xvfb) as a PNG. Usage: grab.py OUT.png
import sys, gi
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk
root = Gdk.get_default_root_window()
w, h = root.get_width(), root.get_height()
Gdk.pixbuf_get_from_window(root, 0, 0, w, h).savev(sys.argv[1], "png", [], [])
