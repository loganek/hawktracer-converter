[![Linux build status](https://travis-ci.org/loganek/hawktracer-converter.svg)](https://travis-ci.org/loganek/hawktracer-converter)
[![Windows build status](https://ci.appveyor.com/api/projects/status/80mxn53hol3su7lm?svg=true)](https://ci.appveyor.com/project/loganek/hawktracer-converter)
[![Crates.io](https://img.shields.io/crates/v/hawktracer-converter.svg)](https://crates.io/crates/hawktracer-converter)

HawkTracer Converter
------------
HawkTracer Converter is a command line tool for converting [HawkTracer](https://www.hawktracer.org) binary stream to well-known tracing format.
Converter supports following output formats:

* FlameGraph HTML files (http://www.brendangregg.com/flamegraphs.html)
* Trace Event Format (https://github.com/catapult-project/catapult/wiki/Trace-Event-Format)
* Debug output - prints raw events in a human-readable form

We strive to support as many formats as possible, so please [create an issue](https://github.com/loganek/hawktracer-converter/issues/new) to let us know that you need another output format to be supported.

## Documentation quick links

* [Installation](#installation)
* [Usage](#usage)
* [Examples](#examples)
* [Building](#building)


## Screenshots of output formats

| Trace Event Format | FlameGraph |
| ------------------ | ---------- |
| [![A screenshot of a Trace Event Format output](https://www.hawktracer.org/img/chrometracing.png)](https://www.hawktracer.org/img/chrometracing.png) | [![A screenshot of a flamegraph output](https://www.hawktracer.org/img/flamegraph.png)](https://www.hawktracer.org/img/flamegraph.png) |

## Installation

### Download binary files
For each release team publishes ready-to-run executables for Linux and Windows operating systems. If you don't have rust environment, and you don't want to build the converter on your own, we recommend to download binaries from the [release page](https://github.com/loganek/hawktracer-converter/releases).  
You should download a file with the following name: `hawktracer-converter-{VERSION}-{OPERATING_SYSTEM}-{ARCHITECTURE}` (optionally with `.exe` extension for Windows platforms), e.g. `hawktracer-converter-0.1.0-linux-x86_64`.

### Cargo Install
If you have [Rust](http://www.rust-lang.org/) developer tools, the easiest way to install the converter is to run cargo install command:
```bash
cargo install hawktracer-converter
```
This command will install `hawktracer-converter` application to user's installation bin root's bin folder (by default it's `$HOME/.cargo/bin`). Make sure that directory is in your `$PATH` to be able to run the application without specifying a full path.

## Usage
```bash
$ hawktracer-converter --help
  USAGE:
    hawktracer-converter [FLAGS] [OPTIONS] --format <format> --output-file <output-file> --source <source>

  FLAGS:
    -h, --help       Prints help information
        --stdout     Print data to standard output
    -V, --version    Prints version information
        --verbose    Print debug information

  OPTIONS:
        --format <format>              Conversion format [possible values: debug, chrome-tracing, flamegraph]
        --map-files <map-files>        List of mapping files
        --output-file <output-file>    Output file [default: hawktracer-trace-%Y-%m-%d-%H_%M_%S.httrace]
        --source <source>              Data source description (either filename, or server address)

```

## Examples

* Read HawkTracer data stream from the network and generate FlameGraph in the default location:
```bash
$ hawktracer-converter --format flamegraph --source 10.16.32.249:5443
```
* Read HawkTracer data file and print raw events to standard output:
```bash
$ hawktracer-converter --format debug --stdout
```

## Building
HawkTracer Converter is implemented in [Rust](https://www.rust-lang.org/), and it's recommended to use `cargo` tool to compile the project:
```bash
$ git clone https://github.com/loganek/hawktracer-converter
$ cd hawktracer-converter
$ cargo build --release
$ ./target/release/hawktracer-converter --version
  hawktracer-converter 0.1.0
```

### 
## License Summary

This project is made available under the MIT license. 
(See [LICENSE](LICENSE) file)