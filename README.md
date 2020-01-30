# Cups CLI

Cups CLI is a terminal based client for interacting with an installed instance of the Cups Messenger sovereign app on a Start9 Server.

On a Mac or Linux based machine you can find the terminal by searching for "terminal" in the global search bar.

In what follows a '$' indicates that the following text is to be pasted into the terminal, followed by return to run the command. You should not copy the '$' sign. Please direct issues with installation to support@start9labs.com.

## Install
### Requires Rust
```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source ~/.cargo/env
```
### Install
```
$ cargo install cups-cli
```

## Usage

The terminal program cups-cli is now installed onto your system, and can be used and invoked by issuing requests of the following form.

```
$ cups-cli <Command> <SubCommand> < ... arguments ... >
```

Here and in what follows the hard angle brackets `< ... >` indicate that you will type one of a handful of options in those locations within the command.

### Configuration

In order to run cups-cli using your Start9 Server, you'll need to set up the cups-cli program to point to your server. This is done by configuring a CUPS_HOST and a CUPS_PASSWORD. 

Open your Start9 Companion App, click on the server running Cups Messenger, click on the "..." hamburger menu in the top right corner, and then "Server Specs". The first line "LAN IP" will be your CUPS_HOST. To get the password, navigate to your installed Cups Messenger app, click the "..." kebab menu and then "App Config". 

To communicate these value to your cups-cli terminal program, run the following in your terminal with no extra spaces or quotes:

```
$ export CUPS_HOST=<your LAN IP>
$ export CUPS_PASSWORD=<your Cups Messager password>
```

### Commands and SubCommands

  - `$ cups-cli contacts ...` : The contacts command allows you to see and add new contacts for you to message.
    - `$ cups-cli contacts show` : To render your contacts to the screen.
    - `$ cups-cli contacts add <Friend's Tor address> <friend's name>` : To add a new contact of someone running Cups Messenger on their S0 Server, you will need the Tor address listed on their Cups Messenger page.
  - `$ cups-cli messages ...` : The messages command allows you to send and view messsages with your contacts.
    - `$ cups-cli messages show < Friends's Tor address >` : To render your messages with your friend.
    - `$ cups-cli messages send < message > < Friends's Tor address >` : To send a message to your friend! 

### Advanced Usage
  - You can include your CUPS_HOST and CUPS_PASSWORD information on a command by command basis using the -h and -p flags respectively.
  - These variables can also be set in your ~/.bash_profile (macOS) or ~/.bashrc (linux) by adding the following two lines:
  ```
  export CUPS_HOST<your LAN IP>
  export CUPS_PASSWORD=<your Cups Messager password>
  ```
  - You can limit the amount of messages returned to you with `$ cups-cli messages show <Friend's Tor address> --limit n` 

## Terminal User Interface
**WORK IN PROGRESS**
Will start up when no subcommand is passed, however is not in a fully functional state
