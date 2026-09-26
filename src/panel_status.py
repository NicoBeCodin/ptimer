"""Optional XApp panel label. Lines on stdin update the label; EOF removes it."""

import sys

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("XApp", "1.0")
from gi.repository import GLib, Gtk, XApp

if not XApp.StatusIcon.any_monitors():
    sys.exit(2)


icon = XApp.StatusIcon()
icon.set_icon_name("alarm-symbolic")
icon.set_label("PTIMER")
icon.set_tooltip_text("PTimer is ready")


def update_from_stdin(_source, _condition):
    line = sys.stdin.readline()
    if not line:
        Gtk.main_quit()
        return False
    label = line.strip()[:32]
    icon.set_label(label)
    icon.set_tooltip_text(f"PTimer: {label}")
    return True


GLib.io_add_watch(sys.stdin, GLib.IO_IN | GLib.IO_HUP, update_from_stdin)
Gtk.main()
