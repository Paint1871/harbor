//! Base64 for IPC payloads. Harbor keeps this in-tree rather than taking a
//! dependency for twenty lines of table lookup.

pub fn encode(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(TABLE[((triple >> 18) & 63) as usize] as char);
        out.push(TABLE[((triple >> 12) & 63) as usize] as char);
        out.push(if i + 1 < bytes.len() {
            TABLE[((triple >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if i + 2 < bytes.len() {
            TABLE[(triple & 63) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    out
}

/// Strict decode for the same alphabet. Anything outside the table, padding
/// in the middle, or a length that is not a multiple of four is refused —
/// renderer input is never trusted enough for a forgiving parser.
pub fn decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(ch: u8) -> Option<u8> {
        match ch {
            b'A'..=b'Z' => Some(ch - b'A'),
            b'a'..=b'z' => Some(ch - b'a' + 26),
            b'0'..=b'9' => Some(ch - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let filtered: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !filtered.len().is_multiple_of(4) {
        return Err("invalid base64".into());
    }
    let pad = filtered.iter().rev().take_while(|b| **b == b'=').count();
    if pad > 2 || filtered[..filtered.len() - pad].contains(&b'=') {
        return Err("invalid base64".into());
    }
    let mut out = Vec::with_capacity(filtered.len() / 4 * 3);
    for chunk in filtered.chunks(4) {
        let v0 = val(chunk[0]).ok_or("invalid base64")?;
        let v1 = val(chunk[1]).ok_or("invalid base64")?;
        let v2 = if chunk[2] == b'=' {
            0
        } else {
            val(chunk[2]).ok_or("invalid base64")?
        };
        let v3 = if chunk[3] == b'=' {
            0
        } else {
            val(chunk[3]).ok_or("invalid base64")?
        };
        let triple = ((v0 as u32) << 18) | ((v1 as u32) << 12) | ((v2 as u32) << 6) | v3 as u32;
        out.push((triple >> 16) as u8);
        if chunk[2] != b'=' {
            out.push((triple >> 8) as u8);
        }
        if chunk[3] != b'=' {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn pads_each_tail_length() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn encodes_bytes_outside_ascii() {
        assert_eq!(encode(&[0x00, 0xff, 0x80]), "AP+A");
    }

    #[test]
    fn decode_roundtrips_every_tail_length() {
        for bytes in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0x00, 0xff, 0x80, 0x7f],
        ] {
            assert_eq!(decode(&encode(bytes)).as_deref(), Ok(bytes));
        }
    }

    #[test]
    fn decode_refuses_malformed_input() {
        for bad in [
            "A",        // not a multiple of four
            "AB!D",     // character outside the table
            "AA=A",     // padding inside a chunk
            "AA==BB==", // padding that is not the tail
            "A===",     // more than two padding bytes
            "=AAA",     // leading padding
        ] {
            assert_eq!(decode(bad), Err("invalid base64".to_string()), "{bad}");
        }
        // Whitespace the encoder never emits is still skipped, and empty input
        // decodes to empty so a blank write stays a no-op.
        assert_eq!(decode(" Zm9v\n").as_deref(), Ok(&b"foo"[..]));
        assert_eq!(decode("").as_deref(), Ok(&b""[..]));
    }
}
