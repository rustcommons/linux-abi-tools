# Clock profile ELF fixtures

Original MIT-licensed x86_64 test data, not runtime implementations. `c.so`
provides the five weak GLIBC_2.17 clock defaults; `rt.so` provides their strong
IFUNC GLIBC_2.2.5 defaults and a non-clock `timer_create` control. Both retain
GNU and SysV symbol hash tables. Tests read the committed binaries portably.

Built with GNU assembler/linker 2.40 (Debian binutils 2.40-2):

```sh
as --64 c.S -o c.o
ld -shared --hash-style=both -soname libc.so.6 --version-script=c.map c.o -o c.so
as --64 rt.S -o rt.o
ld -shared --hash-style=both -soname librt.so.1 --version-script=rt.map rt.o -o rt.so
```

The build here used the equivalent `x86_64-linux-gnu-as` and
`x86_64-linux-gnu-ld` cross tools. `SHA256SUMS` records sources and binaries.
