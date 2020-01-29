# C.U.P.S. CLI

## Install
### Requires Rust
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```
### Install
```
cargo install cups-cli
```

## Usage
### Base arguments (REQUIRED)
  - HOST
    - Must be the LAN IP or .local address of your C.U.P.S. server
    - Can be set with either the `CUPS_HOST` environment var or `-h` command line flag
  - PASSWORD
    - Can be found in your Start9 Server config
    - Can be set with either the `CUPS_PASSWORD` environment var or `-p` command line flag
### Subcommands
  - contacts
    - show/list/ls
      - Shows contact book and unread messages
    - add
      - ADDRESS
        - Must be the .onion address of the contact
      - NAME
        - A friendly name for the contact
  - messages
    - show/list/ls
      - ADDRESS
        - Must be the .onion address of the contact you want to see message history with
      - `--limit` (optional)
        - set maximum number of messages to return
    - send
      - ADDRESS
        - Must be the .onion address of the intended recipient
      - MESSAGE
        - Your message to send. For best results, surround in quotes.

## Terminal User Interface
**WORK IN PROGRESS**
Will start up when no subcommand is passed, however is not in a fully functional state
