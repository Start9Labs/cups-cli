use std::net::IpAddr;

use clap::{App, Arg, SubCommand};
use failure::Error;
use reqwest::blocking as rq;
use sha3::{Digest, Sha3_256};

fn onion_to_pubkey(onion: &str) -> Result<Vec<u8>, Error> {
    let s = onion.split(".").next().unwrap();
    let mut b = base32::decode(base32::Alphabet::RFC4648 { padding: false }, s)
        .ok_or_else(|| failure::format_err!("invalid base32"))?;
    failure::ensure!(b.len() >= 35, "invalid base32 length");
    failure::ensure!(b[34] == 3, "invalid version");
    let pubkey = &b[..32];
    let mut hasher = Sha3_256::new();
    hasher.input(b".onion checksum");
    hasher.input(pubkey);
    hasher.input(&[3]);
    failure::ensure!(&b[32..34] == &hasher.result()[..2], "invalid checksum");
    b.truncate(32);
    Ok(b)
}

fn pubkey_to_onion(pubkey: &[u8]) -> Result<String, Error> {
    if pubkey.len() != 32 {
        failure::bail!("invalid pubkey length")
    }
    let mut hasher = Sha3_256::new();
    hasher.input(b".onion checksum");
    hasher.input(pubkey);
    hasher.input(&[3]);
    let mut onion = Vec::with_capacity(35);
    onion.extend_from_slice(pubkey);
    onion.extend_from_slice(&hasher.result()[..2]);
    onion.push(3);
    Ok(format!(
        "{}.onion",
        base32::encode(base32::Alphabet::RFC4648 { padding: false }, &onion).to_lowercase()
    ))
}

fn inner_main() -> Result<(), Error> {
    let app = App::new("Cups CLI")
        .version("0.1.0")
        .author("Aiden McClelland <me@drbonez.dev>")
        .about("Interact with Cups")
        .arg(
            Arg::with_name("password")
                .long("password")
                .short("p")
                .takes_value(true),
        );
    let host: Option<IpAddr> = std::env::var("HOST").ok().map(|a| a.parse()).transpose()?;
    let app = if host.is_none() {
        app.arg(
            Arg::with_name("host")
                .long("host")
                .short("h")
                .takes_value(true)
                .required(true),
        )
    } else {
        app
    };

    let mut app = app
        .subcommand(
            SubCommand::with_name("contacts")
                .about("Contact Book")
                .subcommand(
                    SubCommand::with_name("show")
                        .alias("list")
                        .alias("ls")
                        .about("Display contact book"),
                )
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add a new user to your contact book")
                        .arg(Arg::with_name("ADDRESS").required(true))
                        .arg(Arg::with_name("NAME").required(true)),
                ),
        )
        .subcommand(
            SubCommand::with_name("messages")
                .about("Messages")
                .subcommand(
                    SubCommand::with_name("show")
                        .alias("list")
                        .alias("ls")
                        .about("Display contact book")
                        .arg(
                            Arg::with_name("ADDRESS")
                                .help("User to show conversation with")
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("limit")
                                .long("limit")
                                .short("l")
                                .takes_value(true)
                                .help("Maximum number of messages to show"),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("send")
                        .arg(Arg::with_name("ADDRESS").required(true))
                        .arg(Arg::with_name("MESSAGE").required(true)),
                ),
        );

    let matches = app.clone().get_matches();
    let password = matches
        .value_of("password")
        .map(|a| a.to_owned())
        .or_else(|| std::env::var("PASSWORD").ok())
        .or_else(|| {
            use std::io::Write;
            print!("PASSWORD: ");
            std::io::stdout().flush().unwrap();
            rpassword::read_password().ok()
        })
        .ok_or_else(|| failure::format_err!("requires password"))?;
    let host: IpAddr = matches
        .value_of("host")
        .map(|a| a.parse())
        .transpose()?
        .or(host)
        .unwrap();
    match matches.subcommand() {
        ("contacts", Some(sub_m)) => match sub_m.subcommand() {
            ("show", _) | ("list", _) | ("ls", _) => {
                use prettytable::{Cell, Row, Table};
                use std::io::Read;

                let mut res = rq::Client::new()
                    .get(&format!("http://{}:59001?type=users", host))
                    .basic_auth("me", Some(&password))
                    .send()?;
                let status = res.status();
                if !status.is_success() {
                    failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
                }

                let mut table = Table::new();
                table.add_row(Row::new(vec![
                    Cell::new("ADDRESS"),
                    Cell::new("NAME"),
                    Cell::new("UNREADS"),
                ]));
                loop {
                    let mut row = Row::empty();
                    let mut buf = [0; 32];
                    match res.read_exact(&mut buf) {
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        a => a?,
                    };
                    row.add_cell(Cell::new(&pubkey_to_onion(&buf)?));
                    let mut buf = [0; 8];
                    res.read_exact(&mut buf)?;
                    let unreads = u64::from_be_bytes(buf);
                    let mut buf = [0];
                    res.read_exact(&mut buf)?;
                    let mut buf = vec![0; buf[0] as usize];
                    res.read_exact(&mut buf)?;
                    row.add_cell(Cell::new(&String::from_utf8(buf)?));
                    row.add_cell(Cell::new(&format!("{}", unreads)));
                    table.add_row(row);
                }
                table.printstd();
            }
            ("add", Some(sub_sub_m)) => {
                let mut req = Vec::new();
                req.push(1);
                req.extend_from_slice(&onion_to_pubkey(&sub_sub_m.value_of("ADDRESS").unwrap())?);
                req.extend_from_slice(sub_sub_m.value_of("NAME").unwrap().as_bytes());
                let status = rq::Client::new()
                    .post(&format!("http://{}:59001", host))
                    .basic_auth("me", Some(&password))
                    .body(req)
                    .send()?
                    .status();
                if !status.is_success() {
                    failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
                }
            }
            _ => {
                app.print_long_help()?;
                println!()
            }
        },
        ("messages", Some(sub_m)) => match sub_m.subcommand() {
            ("show", Some(sub_sub_m)) | ("list", Some(sub_sub_m)) | ("ls", Some(sub_sub_m)) => {
                use prettytable::{Cell, Row, Table};
                use std::io::Read;

                let mut res = rq::Client::new()
                    .get(&if let Some(limit) = sub_sub_m.value_of("limit") {
                        format!(
                            "http://{}:59001?type=messages&pubkey={}&limit={}",
                            host,
                            base32::encode(
                                base32::Alphabet::RFC4648 { padding: false },
                                &onion_to_pubkey(&sub_sub_m.value_of("ADDRESS").unwrap())?
                            )
                            .to_lowercase(),
                            limit
                        )
                    } else {
                        format!(
                            "http://{}:59001?type=messages&pubkey={}",
                            host,
                            base32::encode(
                                base32::Alphabet::RFC4648 { padding: false },
                                &onion_to_pubkey(&sub_sub_m.value_of("ADDRESS").unwrap())?
                            )
                            .to_lowercase()
                        )
                    })
                    .basic_auth("me", Some(&password))
                    .send()?;
                let status = res.status();
                if !status.is_success() {
                    failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
                }

                let mut table = Table::new();
                table.add_row(Row::new(vec![
                    Cell::new("TYPE"),
                    Cell::new("TIME"),
                    Cell::new("MESSAGE"),
                ]));
                let mut msgs = Vec::new();
                loop {
                    let mut row = Row::empty();
                    let mut buf = [0];
                    match res.read_exact(&mut buf) {
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                        a => a?,
                    };
                    row.add_cell(Cell::new(if buf[0] == 0 { "OUTBOUND" } else { "INBOUND" }));
                    let mut buf = [0; 8];
                    res.read_exact(&mut buf)?;
                    let time = i64::from_be_bytes(buf);
                    row.add_cell(Cell::new(&format!(
                        "{}",
                        chrono::DateTime::<chrono::Local>::from(if time > 0 {
                            std::time::UNIX_EPOCH + std::time::Duration::from_secs(time as u64)
                        } else {
                            std::time::UNIX_EPOCH
                                - std::time::Duration::from_secs(time.abs() as u64)
                        })
                    )));
                    let mut buf = [0; 8];
                    res.read_exact(&mut buf)?;
                    let len = u64::from_be_bytes(buf);
                    let mut buf = vec![0; len as usize];
                    res.read_exact(&mut buf)?;
                    row.add_cell(Cell::new(&String::from_utf8(buf)?));
                    msgs.push(row);
                }
                for msg in msgs.into_iter().rev() {
                    table.add_row(msg);
                }
                table.printstd();
            }
            ("send", Some(sub_sub_m)) => {
                let mut req = Vec::new();
                req.push(0);
                req.extend_from_slice(&onion_to_pubkey(&sub_sub_m.value_of("ADDRESS").unwrap())?);
                req.extend_from_slice(sub_sub_m.value_of("MESSAGE").unwrap().as_bytes());
                let status = rq::Client::new()
                    .post(&format!("http://{}:59001", host))
                    .basic_auth("me", Some(&password))
                    .body(req)
                    .send()?
                    .status();
                if !status.is_success() {
                    failure::bail!(
                        "{}",
                        status.canonical_reason().unwrap_or("UNKNOWN STATUS CODE")
                    );
                }
            }
            _ => {
                app.print_long_help()?;
                println!();
            }
        },
        _ => {
            app.print_long_help()?;
            println!();
        }
    }

    Ok(())
}

fn main() {
    match inner_main() {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }
}
