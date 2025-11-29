# Logalyzer
Logalyzer is a simple tool to help with reading and analyzing log files.

![build](https://github.com/nottomw/logalyzer/actions/workflows/logalyzer.yml/badge.svg)
![tests](https://github.com/nottomw/logalyzer/actions/workflows/logalyzer-test.yml/badge.svg)

## Features
- Customizable log format coloring
- Searching and filtering
- Custom token highlighting
- Histogram visualization of terms in logs
- Commenting on log lines

## Usage
Run as any other application.

There are command line options available, run with `--help` to see them all:
```bash
logalyzer --help
```

## Build
To build Logalyzer, ensure you have [rust](https://rustup.rs/) installed, then run:
```bash
cargo build --release
```
