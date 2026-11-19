# ELF dynamic symbol fixture

`symbols.so` is original MIT-licensed test data assembled from `symbols.S` and
`symbols.map` with GNU binutils. It has no libc dependency. A controlled fixture
program may use it as the real runtime counterpart to a generated link-only DSO;
the generated counterpart is never used as a runtime library.
It exercises default/nondefault versions, weak binding, protected visibility,
same-range observations, object and TLS sizes, IFUNC, and undefined imports.

Run `bash tests/fixtures/elf/build.sh` on x86_64 Linux to regenerate it.
The original assembler/linker versions remain historical in `tools.txt`.
Source-only hashes are recorded in `SHA256SUMS`; tests mutate the generated
tables and check every truncation.

## Source-built fixtures (2026-11-18)

The binaries described above are now generated from these same reviewed sources,
not committed. Run `python3 scripts/build-test-fixtures.py` from the repository
root before tests. The source-only SHA256SUMS remains; generated tool/source/output
identities go to ignored `tests/fixtures/generated-manifest.json`. The historical
hashes, portable CI artifact workflow and local Windows prerequisites are in
[the security guide](../../../docs/SECURITY-BASELINE.md#retired-committed-binary-inventory).
