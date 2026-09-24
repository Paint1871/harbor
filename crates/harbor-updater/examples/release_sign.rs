//! Signs a release artifact with the release secret key — the `release.yml`
//! pipeline calls this instead of shipping a minisign binary. The secret-key
//! box comes from a file, the password from `MINISIGN_PASSWORD`, so neither
//! ends up on a process command line.
//!
//!     cargo run -p harbor-updater --example release_sign -- <key> <file> <sig-out>

use std::io::Cursor;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let key_file = args
        .next()
        .ok_or("usage: release_sign <key-box-file> <file> <sig-out>")?;
    let file = args.next().ok_or("missing <file>")?;
    let sig_out = args.next().ok_or("missing <sig-out>")?;
    let password =
        std::env::var("MINISIGN_PASSWORD").map_err(|_| "MINISIGN_PASSWORD is not set")?;
    let sk_box = minisign::SecretKeyBox::from_string(
        &std::fs::read_to_string(&key_file).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let sk = sk_box
        .into_secret_key(Some(password))
        .map_err(|error| error.to_string())?;
    let data = std::fs::read(&file).map_err(|error| error.to_string())?;
    let signature = minisign::sign(None, &sk, Cursor::new(data), None, None)
        .map_err(|error| error.to_string())?;
    std::fs::write(&sig_out, signature.into_string()).map_err(|error| error.to_string())
}
