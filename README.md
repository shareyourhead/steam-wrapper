# Overview

Steam wrapper is a mouse input mapping program that allows users to define different control profiles via json. The program can be included in the launch options of any game on Steam to launch the desired mapping profile automatically. This provides better support for uncommon mouse buttons, and extends the input options of games with limited rebinding capabilities.

[Software Demo Video](https://youtu.be/uxtQbwNSJJw)

## Usage

Place json5 files in same directory as `steam-wrapper`.

Syntax: `steam-wrapper <profile> [--wrap %command%] [--print-noisy]`

* **`<profile>`** treats "profile.json5" and "profile" the same

* **`[--wrap %command%]`** if you are running `steam-wrapper` via steam launch options

* **`[--print-noisy]`** print relative inputs even if they are marked as noisy. It is recommended that `analog_up` etc. be marked as noisy inside json5

# Development Environment

This program is written in rust. It depends the evdev package on linux to intercept inputs. It uses wayland for window detection, so either use a distro like Ubuntu 25+ or avoid using the --wrap flag.

# Useful Websites

- [evdev key codes](https://docs.rs/evdev/latest/evdev/struct.KeyCode.html)
- [evdev relative axis codes](https://docs.rs/evdev/latest/evdev/struct.RelativeAxisCode.html)
- [keyboard checker](https://keyboardchecker.com/)

# Future Work

- Finish implementing all advanced mapping methods defined in [cairn.json5](./profiles/cairn.json5)
- Reduce size of binary
- Write exhaustive documentation for profile.json5 format
- Develop GUI for easy profile creation
