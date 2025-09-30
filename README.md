# picocom - Simple serial communication

The tried and true `minicom` is sooo annoying to use as a beginner. It has too many options and an unintuitive interface. Especially when most of the time you just wanna pick a serial port and view what's coming out. That's where `picocom` comes in!

- Simple interactive CLI
- No options (for now)
- No BS
- Blazingly fast 🔥⚡️ (of course)

## Usage

To interactively choose the serial port:

```sh
picocom
```

To use a specific serial port:

```sh
picocom /dev/my-serial-port
```

## Installation

For now, if you want to install this, it'll be through git:

```sh
cargo install --git https://github.com/tsar-boomba/picocom
```
