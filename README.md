# Wabot-rs
A discord bot in Rust to fetch steps from Wolfram|Alpha, and compile LaTeX snippets

- [Wabot-rs](#wabot-rs)
  - [Build](#build)
    - [Dependencies](#dependencies)
    - [To build](#to-build)
  - [Usage](#usage)
  - [Goals](#goals)
    - [Priority](#priority)
    - [*Might* implement](#might-implement)
  - [Contributing](#contributing)
  - [License](#license)

## Build

This project runs only on Linux, with no plans to support any other OS currently.

### Dependencies
Assuming you already have Rust and Cargo installed,
+ Latex
+ mathjax-node-cli
+ dvisvgm

### To build
Use Cargo

## Usage
With all the dependencies installed,
```sh
cargo run
```
starts the bot.

The bot reads the file `config.toml` for all its variables.
Look at [`config-template.toml`](config-template.toml) to look at an example.

## Goals
### Priority
+ [x] Latex snippets
+ [x] AsciiMath snippets
+ [ ] Images from Wolfram|Alpha
+ [ ] Wolfram|Alpha to AsciiMath/MathML
### *Might* implement
+ [ ] Basic calculator
+ [ ] Graphing

## Contributing
Pull requestes are welcome!

If you want to make major changes, make sure to open an issue first.

## License
This code is licensed under [GPLv3](https://choosealicense.com/licenses/gpl-3.0)