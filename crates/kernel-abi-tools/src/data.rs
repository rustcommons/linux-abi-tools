//! Internal decoding shared by the embedded syscall/madvise tables and fuzzing.
use linux_targets::KernelVersion;

pub struct Row<'a> {
    pub number: u32,
    pub name: &'a str,
    pub first: KernelVersion,
}

pub fn parse(text: &str) -> Result<Vec<Row<'_>>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .enumerate()
        .map(|(index, line)| {
            let fields: Vec<_> = line.split('\t').map(str::trim).collect();
            let [number, name, first] = fields[..] else {
                return Err(format!("kernel ABI data row {} needs 3 columns", index + 1));
            };
            Ok(Row {
                number: number
                    .parse()
                    .map_err(|_| format!("invalid kernel ABI number {number:?}"))?,
                name,
                first: KernelVersion::parse(first)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_tables_and_rejects_malformed_rows_without_panics() {
        let rows = parse("# provenance\n0\tread\t3.3\n9\tMADV_REMOVE\tv2.6.16\n").unwrap();
        assert_eq!(
            (rows[0].number, rows[0].name, rows[0].first.to_string()),
            (0, "read", "3.3".into())
        );
        assert_eq!(rows[1].first.to_string(), "2.6");
        for bad in [
            "0\tread",
            "0\tread\t3.3\textra",
            "4294967296\tread\t3.3",
            "-1\tread\t3.3",
            "0\tread\tbogus",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }
}
