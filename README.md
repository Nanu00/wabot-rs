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
    - [Not implementing](#not-implementing)
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

## Config files
The bot expects `~/.config/wally/` to exist, along with a `config.ron` file containing the Discord token and application id.
Apart from `config.ron`, there are also per-module config files.

The file format of the config files is [RON](https://docs.rs/ron/0.6.4/ron/). The format is defined inside `lib.rs` and inside the module if needs a config.

## Goals
### Priority
+ [x] Latex snippets
+ [x] AsciiMath snippets
+ [x] Images from Wolfram|Alpha
+ [ ] Logging
+ [ ] Slash commands
### *Might* implement
+ [ ] Basic calculator

### Not implementing
+ Wolfram|Alpha to AsciiMath/MathML
  + The ASCIIMath recieved from the API is a pain to parse
  + Some of the symbols in the MathML recieved from the API are not properly displayed by MathJax
+ Graphing
  + Doesn't really fit what the bot is supposed to do
  + Can already be done using Wolfram|Alpha

## Contributing
Pull requestes are welcome!

If you want to make major changes, make sure to open an issue first.

## License
This code is licensed under [GPLv3](https://choosealicense.com/licenses/gpl-3.0)
