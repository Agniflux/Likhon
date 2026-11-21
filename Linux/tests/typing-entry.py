#!/usr/bin/env python3
"""A one-line GTK text box for the headless typing test. Writes what it holds to the file given as argument, every
time it changes, so the test can read the result. Not shipped.

    typing-entry.py OUT_FILE [SECONDS] [--password]

--password hides the text (GTK then tells the input method it is a password field)."""
import sys

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

args = [a for a in sys.argv[1:] if not a.startswith("--")]
password = "--password" in sys.argv
out = args[0]
seconds = int(args[1]) if len(args) > 1 else 20

win = Gtk.Window(title="likhon-test")
entry = Gtk.Entry()
if password:
    entry.set_visibility(False)
win.add(entry)
win.set_default_size(400, 60)
win.connect("destroy", Gtk.main_quit)


def save(*_):
    with open(out, "w", encoding="utf-8") as f:
        f.write(entry.get_text())


entry.connect("changed", save)
win.show_all()
entry.grab_focus()
save()
GLib.timeout_add_seconds(seconds, Gtk.main_quit)
Gtk.main()
