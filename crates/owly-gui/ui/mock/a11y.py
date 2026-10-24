#!/usr/bin/env python3
"""Runs axe-core against each mock scenario in WebKitGTK, the engine Tauri uses
on Linux, in light and dark. Needs python3-gi, python3-gi-cairo,
gir1.2-webkit2-4.1 and xvfb, and the vite dev server on port 1420:

    npm run dev &
    xvfb-run -a python3 mock/a11y.py

Exits 1 when axe reports a violation.
"""
import json
import os
import sys
from pathlib import Path

# Software rendering: Xvfb has no GL.
os.environ["WEBKIT_DISABLE_COMPOSITING_MODE"] = "1"
os.environ["WEBKIT_DISABLE_DMABUF_RENDERER"] = "1"
os.environ["GDK_BACKEND"] = "x11"

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2

BASE = "http://localhost:1420/mock/mock.html"
# (scenario, script to run once it has loaded, e.g. to open a dialog)
SCENARIOS = [
    ("off", ""),
    ("empty", ""),
    ("people", ""),
    ("transfer", ""),
    ("incoming", ""),
    ("files", ""),
    ("problem", ""),
    ("people", "document.querySelector('.person').click()"),
    ("doctor", "document.querySelectorAll('nav button')[1].click()"),
]
AXE = (Path(__file__).parent.parent / "node_modules/axe-core/axe.min.js").read_text()
# Titles are cut short by WebKit, so the page sends a compact summary.
RUN = (
    "axe.run(document).then(r => { document.title = 'AXE:' + JSON.stringify("
    "r.violations.slice(0, 4).map(v => ({id: v.id, help: v.help.slice(0, 60),"
    " nodes: v.nodes.slice(0, 2).map(n => ({target: String(n.target).slice(0, 50)}))})))"
    "; });"
)


def check(scenario: str, action: str, dark: bool) -> list:
    Gtk.Settings.get_default().set_property("gtk-application-prefer-dark-theme", dark)
    window = Gtk.OffscreenWindow()
    window.set_default_size(960, 600)
    view = WebKit2.WebView()
    window.add(view)
    window.show_all()
    found: list = []

    def run(script: str) -> bool:
        view.evaluate_javascript(script, -1, None, None, None, None, None)
        return False

    def read() -> bool:
        # The page puts axe's answer in its title; no answer is a failure.
        title = view.get_title() or ""
        try:
            if not title.startswith("AXE:"):
                raise ValueError(title)
            found.extend(json.loads(title[4:]))
        except ValueError:
            found.append({"id": "no-result", "help": "axe didn't finish", "nodes": []})
        Gtk.main_quit()
        return False

    def loaded(_view, event):
        if event == WebKit2.LoadEvent.FINISHED:
            GLib.timeout_add(1200, run, action or "0")
            GLib.timeout_add(2000, run, AXE + ";" + RUN)
            GLib.timeout_add(4500, read)

    view.connect("load-changed", loaded)
    view.load_uri(f"{BASE}#{scenario}")
    GLib.timeout_add(20000, Gtk.main_quit)
    Gtk.main()
    window.destroy()
    return found


def main() -> int:
    failed = 0
    for dark in (False, True):
        for scenario, action in SCENARIOS:
            opened = " (dialog opened)" if action and "person" in action else " (tab)" if action else ""
            label = f"{scenario}{opened} ({'dark' if dark else 'light'})"
            violations = check(scenario, action, dark)
            if violations:
                failed += 1
            print(("FAIL " if violations else "ok   ") + label)
            for v in violations:
                print(f"       {v['id']}: {v['help']}")
                for node in v["nodes"][:3]:
                    print(f"         {node['target']}")
    return 1 if failed else 0


sys.exit(main())
