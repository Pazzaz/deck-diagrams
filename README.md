# Deck Diagrams
CLI tool to download deck data from [EDHREC](https://edhrec.com/) and generate diagrams (SVG or PNG).

## Example
![Example diagram](examples/doctor_who.svg)

## Usage
Output of `deck-diagrams --help`:
```
CLI to download statistics from EDHREC and generate diagrams

Usage: deck-diagrams <DATA> <COMMAND>

Commands:
  download  Download statistics
  render    Render SVG diagram
  help      Print this message or the help of the given subcommand(s)

Arguments:
  <DATA>  Path to JSON file containing statistics

Options:
  -h, --help  Print help
```