# Overview

Steam wrapper is a complex rebinding program that...
For use on Ubuntu 25+

{Important! Do not say in this section that this is college assignment. Talk about what you are trying to accomplish as a software engineer to further your learning.}

{Provide a description of the software that you wrote to demonstrate the Rust language.}

{Describe your purpose for writing this software.}

{Provide a link to your YouTube demonstration. It should be a 4-5 minute demo of the software running and a walkthrough of the code. Focus should be on sharing what you learned about the language syntax.}

[Software Demo Video](http://youtube.link.goes.here)

## Usage

Place json5 files in same location as `steam-wrapper`.

Syntax: `steam-wrapper <profile> [--wrap %command%] [--print-noisy]`

* **`<profile>`** treats "profile.json5" and "profile" the same

* **`[--wrap %command%]`** if you are running `steam-wrapper` via steam launch options

* **`[--print-noisy]`** print relative inputs even if they are marked as noisy. It is recommended that `analog_up` etc. be marked as noisy inside json5

# Development Environment

{Describe the tools that you used to develop the software}

{Describe the programming language that you used and any libraries.}

# Useful Websites

{Make a list of websites that you found helpful in this project}

- [evdev key codes](https://docs.rs/evdev/latest/evdev/struct.KeyCode.html)
- [evdev relative axis codes](https://docs.rs/evdev/latest/evdev/struct.RelativeAxisCode.html)
- [keyboard checker](https://keyboardchecker.com/)

# Future Work

{Make a list of things that you need to fix, improve, and add in the future.}

- Write exhaustive documentation for profile.json5 format
- Develop GUI for easy profile creation
- Item 3
