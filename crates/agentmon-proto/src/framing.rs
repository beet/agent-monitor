use std::io::{self, BufRead, Write};

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Writes `message` as a single line of JSON, newline-terminated.
pub fn write_message<W: Write, T: Serialize>(writer: &mut W, message: &T) -> io::Result<()> {
    let json = serde_json::to_string(message)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Reads one newline-delimited JSON message.
///
/// Returns `Ok(None)` on a clean EOF (the peer closed the connection without
/// sending a message). Returns `Err` for a malformed payload; callers decide
/// whether that should end the connection.
pub fn read_message<R: BufRead, T: DeserializeOwned>(reader: &mut R) -> io::Result<Option<T>> {
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;
    if bytes_read == 0 {
        return Ok(None);
    }

    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        return Ok(None);
    }

    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn write_then_read_round_trips_a_message() {
        let mut buffer = Vec::new();
        write_message(&mut buffer, &Sample { value: 42 }).unwrap();

        let mut reader = BufReader::new(buffer.as_slice());
        let decoded: Sample = read_message(&mut reader).unwrap().unwrap();

        assert_eq!(decoded, Sample { value: 42 });
    }

    #[test]
    fn read_message_returns_none_on_empty_input() {
        let mut reader = BufReader::new([].as_slice());

        let decoded: Option<Sample> = read_message(&mut reader).unwrap();

        assert_eq!(decoded, None);
    }

    #[test]
    fn read_message_errors_on_malformed_json() {
        let mut reader = BufReader::new(b"not json\n".as_slice());

        let result: io::Result<Option<Sample>> = read_message(&mut reader);

        assert!(result.is_err());
    }
}
