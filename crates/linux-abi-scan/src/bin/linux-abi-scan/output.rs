//! Tool-owned stdout writes. Never suppress command/child-process errors.
use std::io::{self, Write};

pub(crate) fn emit(text: &str) -> Result<(), String> {
    write_output(&mut io::stdout().lock(), text)
}

fn write_output(out: &mut impl Write, text: &str) -> Result<(), String> {
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
            Err(format!("cannot write to stdout: {error}"))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Failure {
        kind: io::ErrorKind,
        on_flush: bool,
    }
    impl Write for Failure {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.on_flush {
                Ok(bytes.len())
            } else {
                Err(self.kind.into())
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(self.kind.into())
        }
    }

    #[test]
    fn write_and_flush_suppress_only_broken_pipe() {
        for on_flush in [false, true] {
            for kind in [
                io::ErrorKind::BrokenPipe,
                io::ErrorKind::PermissionDenied,
                io::ErrorKind::WriteZero,
            ] {
                let result = write_output(&mut Failure { kind, on_flush }, "output");
                assert_eq!(result.is_ok(), kind == io::ErrorKind::BrokenPipe);
            }
        }
    }

    #[test]
    fn buffered_write_is_flushed_before_returning() {
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::PermissionDenied] {
            let mut out = io::BufWriter::new(Failure {
                kind,
                on_flush: false,
            });
            // Fits in the buffer: only the explicit flush reaches the failing sink.
            assert_eq!(
                write_output(&mut out, "small").is_ok(),
                kind == io::ErrorKind::BrokenPipe
            );
        }
        let mut bytes = Vec::new();
        write_output(&mut io::BufWriter::new(&mut bytes), "unchanged\n").unwrap();
        assert_eq!(bytes, b"unchanged\n");
    }
}
