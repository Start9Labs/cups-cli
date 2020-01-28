# C.U.P.S. CLI

## Building
### Requires Rust
```curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh```
### Build
```cargo build --release```
### Add to PATH
```cp ./target/release/cups-cli /usr/local/bin```

## Usage
### Base arguments
  - HOST
    - Must be the LAN IP address of your C.U.P.S. server
    - Can be set with either the `HOST` environment var or `-h` command line flag
  - PASSWORD
    - Can be found in your Start9 Server config
    - Can be set with either the `PASSWORD` environment var or `-p` command line flag
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
