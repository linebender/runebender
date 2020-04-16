# Runebender

A font editor written in Rust, currently in very early stages.

**This software is very raw, and is not yet suitable for use**.

## Building

### macOS

You need to have `libcairo` installed.
There is currently an [issue](https://github.com/gtk-rs/cairo/issues/263) when `libcairo` is installed via `homebrew`.

Until this is fixed please compile using

    PKG_CONFIG_PATH="/usr/local/opt/libffi/lib/pkgconfig" cargo build

## Running

To run on a UFO file:

```rust
cargo run -- optional/path/to/some/unifedfontobject.ufo
```

UFO files may also be opened with file>open in the running program.

## Contributions

Contributions are welcome. The [Rust Code of Conduct] applies. Please feel free to add your name to the [AUTHORS] file in any substantive pull request.

A very good place to ask questions and discuss development work is our
[Zulip chat instance](https://xi.zulipchat.com), in the [#runebender](https://xi.zulipchat.com/#narrow/stream/197829-runebender) channel.

## License

All files in this repository are licensed under the [Apache 2.0](LICENSE) license.

[Rust Code of Conduct]: https://www.rust-lang.org/policies/code-of-conduct
[AUTHORS]: AUTHORS
