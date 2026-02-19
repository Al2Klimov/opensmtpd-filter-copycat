use std::io::{Result, Write};

pub(crate) fn join_write_bytes<'a>(
    writer: &mut dyn Write,
    sep: &[u8],
    mut parts: impl Iterator<Item = &'a [u8]>,
) -> Result<()> {
    match parts.next() {
        None => {}
        Some(first) => {
            writer.write_all(first)?;

            for part in parts {
                writer.write_all(sep)?;
                writer.write_all(part)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_write_bytes_empty() {
        let mut out = Vec::new();
        join_write_bytes(&mut out, b"|", std::iter::empty::<&[u8]>()).unwrap();
        assert_eq!(out, b"");
    }

    #[test]
    fn test_join_write_bytes_single() {
        let mut out = Vec::new();
        join_write_bytes(&mut out, b"|", [b"hello".as_ref()].into_iter()).unwrap();
        assert_eq!(out, b"hello");
    }

    #[test]
    fn test_join_write_bytes_multiple() {
        let mut out = Vec::new();
        join_write_bytes(
            &mut out,
            b"|",
            [b"a".as_ref(), b"b".as_ref(), b"c".as_ref()].into_iter(),
        )
        .unwrap();
        assert_eq!(out, b"a|b|c");
    }

    #[test]
    fn test_join_write_bytes_multi_byte_sep() {
        let mut out = Vec::new();
        join_write_bytes(&mut out, b", ", [b"x".as_ref(), b"y".as_ref()].into_iter()).unwrap();
        assert_eq!(out, b"x, y");
    }
}
