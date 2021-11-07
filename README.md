# Win-Rearrange
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/TumbleOwlee/ws-dissector-lib/blob/master/LICENSE)
![Ubuntu Build](https://github.com/TumbleOwlee/win-rearrange/actions/workflows/rust.yml/badge.svg)

Win-Rearrange is a simple tool to manipulate the position and size of any window in X11. It also allows to hide, show and raise any window. By providing a regex that has to match to the window title you can use the provided commands to change the window's state. I create this tool to know what is necessary to provide these functionalities. It is based on libX11 and thus only supports XServer at the moment. The next goal is to also provide these commands on Windows 10.
