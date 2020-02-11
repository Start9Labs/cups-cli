use clap::{App, Arg, SubCommand};
use failure::Error;
use reqwest::Proxy;
use url::Host;

#[cfg(feature = "tui")]
mod tui;

async fn inner_main() -> Result<(), Error> {
    let host: Option<Host> = std::env::var("CUPS_HOST")
        .ok()
        .map(|a| Host::parse(&a))
        .transpose()?;
    let proxy: Option<Proxy> = std::env::var("CUPS_PROXY")
        .ok()
        .map(|a| Proxy::http(&format!("socks5h://{}:9050", a)))
        .transpose()?;

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
                        .about("Display messages with a user")
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
        .or_else(|| std::env::var("CUPS_PASSWORD").ok())
        .or_else(|| {
            use std::io::Write;
            print!("PASSWORD: ");
            std::io::stdout().flush().unwrap();
            rpassword::read_password().ok()
        })
        .ok_or_else(|| failure::format_err!("requires password"))?;
    let host: Host = matches
        .value_of("host")
        .map(Host::parse)
        .transpose()?
        .or(host)
        .unwrap();
    let proxy = match &host {
        Host::Domain(s) if s.ends_with(".onion") => Some(Proxy::http("socks5h://127.0.0.1:9050")?),
        _ => proxy,
    };
    let creds = cupslib::Creds {
        host,
        proxy,
        password,
    };
    match matches.subcommand() {
        ("contacts", Some(sub_m)) => match sub_m.subcommand() {
            ("show", _) | ("list", _) | ("ls", _) => {
                use prettytable::{Cell, Row, Table};

                let mut table = Table::new();
                table.add_row(Row::new(vec![
                    Cell::new("ADDRESS"),
                    Cell::new("NAME"),
                    Cell::new("UNREADS"),
                ]));
                for user in cupslib::fetch_users(&creds).await? {
                    table.add_row(Row::new(vec![
                        Cell::new(&cupslib::pubkey_to_onion(&user.id)?),
                        Cell::new(&user.name.as_ref().map(|a| a.as_str()).unwrap_or("")),
                        Cell::new(&format!("{}", user.unreads)),
                    ]));
                }
                table.printstd();
            }
            ("add", Some(sub_sub_m)) => {
                cupslib::add_user(
                    &creds,
                    sub_sub_m.value_of("ADDRESS").unwrap(),
                    sub_sub_m.value_of("NAME").unwrap(),
                )
                .await?
            }
            _ => {
                app.print_long_help()?;
                println!()
            }
        },
        ("messages", Some(sub_m)) => match sub_m.subcommand() {
            ("show", Some(sub_sub_m)) | ("list", Some(sub_sub_m)) | ("ls", Some(sub_sub_m)) => {
                use prettytable::{Cell, Row, Table};

                let mut table = Table::new();
                table.add_row(Row::new(vec![
                    Cell::new("TYPE"),
                    Cell::new("TIME"),
                    Cell::new("MESSAGE"),
                ]));
                let msgs = cupslib::fetch_messages(
                    &creds,
                    &cupslib::onion_to_pubkey(sub_sub_m.value_of("ADDRESS").unwrap())?,
                    sub_sub_m.value_of("limit").map(|a| a.parse()).transpose()?,
                )
                .await?;
                for msg in msgs.into_iter().rev() {
                    table.add_row(Row::new(vec![
                        Cell::new(if msg.inbound { "INBOUND" } else { "OUTBOUND" }),
                        Cell::new(&format!(
                            "{}",
                            chrono::DateTime::<chrono::Local>::from(if msg.time > 0 {
                                std::time::UNIX_EPOCH
                                    + std::time::Duration::from_secs(msg.time as u64)
                            } else {
                                std::time::UNIX_EPOCH
                                    - std::time::Duration::from_secs(msg.time.abs() as u64)
                            })
                        )),
                        Cell::new(&msg.content),
                    ]));
                }
                table.printstd();
            }
            ("send", Some(sub_sub_m)) => {
                cupslib::send_message(
                    &creds,
                    cupslib::onion_to_pubkey(sub_sub_m.value_of("ADDRESS").unwrap())?.as_ref(),
                    sub_sub_m.value_of("MESSAGE").unwrap(),
                )
                .await?
            }
            _ => {
                app.print_long_help()?;
                println!();
            }
        },
        _ => {
            #[cfg(feature = "tui")]
            tui::tui(creds).await?;
            #[cfg(not(feature = "tui"))]
            {
                app.print_long_help()?;
                println!();
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match inner_main().await {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }
}
