use std::net::IpAddr;

use failure::Error;
use reqwest as rq;
use sha3::{Digest, Sha3_256};

pub struct Pubkey(pub [u8; 32]);
impl AsRef<[u8; 32]> for Pubkey {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

pub fn onion_to_pubkey(onion: &str) -> Result<Pubkey, Error> {
    let s = onion.split(".").next().unwrap();
    let b = base32::decode(base32::Alphabet::RFC4648 { padding: false }, s)
        .ok_or_else(|| failure::format_err!("invalid base32"))?;
    failure::ensure!(b.len() >= 35, "invalid base32 length");
    failure::ensure!(b[34] == 3, "invalid version");
    let pubkey = &b[..32];
    let mut hasher = Sha3_256::new();
    hasher.input(b".onion checksum");
    hasher.input(pubkey);
    hasher.input(&[3]);
    failure::ensure!(&b[32..34] == &hasher.result()[..2], "invalid checksum");
    let mut pk = [0; 32];
    pk.clone_from_slice(pubkey);
    Ok(Pubkey(pk))
}

pub fn pubkey_to_onion(pubkey: &[u8]) -> Result<String, Error> {
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

#[derive(Clone, Debug)]
pub struct Creds {
    pub host: IpAddr,
    pub password: String,
}
impl AsRef<Creds> for Creds {
    fn as_ref(&self) -> &Creds {
        self
    }
}

#[derive(Clone, Debug)]
pub struct UserData {
    pub id: [u8; 32],
    pub name: Option<String>,
    pub unreads: u64,
}

pub async fn fetch_users<C: AsRef<Creds>>(creds: C) -> Result<Vec<UserData>, Error> {
    use std::io::Read;
    let mut users = Vec::new();

    let res = rq::Client::new()
        .get(&format!("http://{}:59001?type=users", creds.as_ref().host))
        .basic_auth("me", Some(&creds.as_ref().password))
        .send()
        .await?;
    let status = res.status();
    if !status.is_success() {
        failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
    }
    let mut b = std::io::Cursor::new(res.bytes().await?);
    loop {
        let mut id = [0; 32];
        match b.read_exact(&mut id) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            a => a?,
        };
        let mut buf = [0; 8];
        b.read_exact(&mut buf)?;
        let unreads = u64::from_be_bytes(buf);
        let mut buf = [0];
        b.read_exact(&mut buf)?;
        let name = if buf[0] == 0 {
            None
        } else {
            let mut buf = vec![0; buf[0] as usize];
            b.read_exact(&mut buf)?;
            Some(String::from_utf8(buf)?)
        };
        users.push(UserData { id, name, unreads })
    }
    Ok(users)
}

pub async fn add_user(creds: &Creds, onion: &str, name: &str) -> Result<(), Error> {
    let mut req = Vec::new();
    req.push(1);
    req.extend_from_slice(onion_to_pubkey(onion)?.as_ref());
    req.extend_from_slice(name.as_bytes());
    let status = rq::Client::new()
        .post(&format!("http://{}:59001", creds.host))
        .basic_auth("me", Some(&creds.password))
        .body(req)
        .send()
        .await?
        .status();
    if !status.is_success() {
        failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Message {
    pub inbound: bool,
    pub time: i64,
    pub content: String,
}

pub async fn fetch_messages<C: AsRef<Creds>, I: AsRef<[u8; 32]>>(
    creds: C,
    id: I,
    limit: Option<usize>,
) -> Result<Vec<Message>, Error> {
    use std::io::Read;

    let mut msgs = Vec::new();
    let res = rq::Client::new()
        .get(&if let Some(limit) = limit {
            format!(
                "http://{}:59001?type=messages&pubkey={}&limit={}",
                creds.as_ref().host,
                base32::encode(base32::Alphabet::RFC4648 { padding: false }, id.as_ref())
                    .to_lowercase(),
                limit
            )
        } else {
            format!(
                "http://{}:59001?type=messages&pubkey={}",
                creds.as_ref().host,
                base32::encode(base32::Alphabet::RFC4648 { padding: false }, id.as_ref())
                    .to_lowercase()
            )
        })
        .basic_auth("me", Some(&creds.as_ref().password))
        .send()
        .await?;
    let status = res.status();
    if !status.is_success() {
        failure::bail!("{}", status.canonical_reason().unwrap_or("UNKNOWN STATUS"));
    }
    let mut b = std::io::Cursor::new(res.bytes().await?);

    loop {
        let mut buf = [0];
        match b.read_exact(&mut buf) {
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            a => a?,
        };
        let inbound = buf[0] != 0;
        let mut buf = [0; 8];
        b.read_exact(&mut buf)?;
        let time = i64::from_be_bytes(buf);
        let mut buf = [0; 8];
        b.read_exact(&mut buf)?;
        let len = u64::from_be_bytes(buf);
        let mut buf = vec![0; len as usize];
        b.read_exact(&mut buf)?;
        msgs.push(Message {
            inbound,
            time,
            content: String::from_utf8(buf)?,
        });
    }

    Ok(msgs)
}

pub async fn send_message(creds: &Creds, id: &[u8; 32], content: &str) -> Result<(), Error> {
    let mut req = Vec::new();
    req.push(0);
    req.extend_from_slice(id);
    req.extend_from_slice(content.as_bytes());
    let status = rq::Client::new()
        .post(&format!("http://{}:59001", creds.host))
        .basic_auth("me", Some(&creds.password))
        .body(req)
        .send()
        .await?
        .status();
    if !status.is_success() {
        failure::bail!(
            "{}",
            status.canonical_reason().unwrap_or("UNKNOWN STATUS CODE")
        );
    }
    Ok(())
}
