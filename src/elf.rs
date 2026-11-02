//! Bounded ELF64 x86_64 dynamic-symbol evidence. No alias semantics are inferred.
//!
//! Layout follows the System V ABI and GNU version-definition extensions.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::File,
    io::Read,
    path::Path,
};

/// The largest ELF file [`Metadata::load`] and [`Metadata::parse`] accept (64 MiB).
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const DYNSYM: u32 = 11;
const STRTAB: u32 = 3;
const DYNAMIC: u32 = 6;
const VERDEF: u32 = 0x6fff_fffd;
const VERSYM: u32 = 0x6fff_ffff;
const TSV_HEADER: &str = "soname\tsymbol\tversion\tdefault\ttype\tbinding\tvisibility\tsection\tsection_alignment\tvalue\tsize\tsame_range\n";

/// One defined dynamic symbol of a shared object, with its GNU symbol version.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Symbol {
    /// The symbol name, without a version suffix.
    pub name: String,
    /// The version node's name, such as `GLIBC_2.17`; `None` when unversioned.
    pub version: Option<String>,
    /// None for unversioned symbols; false for GNU nondefault versions.
    pub default_version: Option<bool>,
    /// The ELF symbol type (`STT_*`): 1 object, 2 function, 6 TLS, 10 IFUNC.
    pub symbol_type: u8,
    /// The ELF binding (`STB_*`): 1 global, 2 weak.
    pub binding: u8,
    /// The ELF visibility (`STV_*`): 0 default, 3 protected.
    pub visibility: u8,
    /// The defining section's index (`st_shndx`).
    pub section: u16,
    /// Declared alignment of the defining section; zero for special indexes.
    pub section_alignment: u64,
    /// The symbol's value (`st_value`): its address in the file's layout.
    pub value: u64,
    /// The symbol's size in bytes (`st_size`).
    pub size: u64,
    /// Same section/value/type/size in this file, not a proven semantic alias.
    pub same_range: Option<usize>,
}

/// The defined, versioned dynamic symbols of one shared object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metadata {
    /// The object's `DT_SONAME`, such as `libc.so.6`.
    pub soname: String,
    /// Its defined dynamic symbols, one entry per name and version.
    pub symbols: Vec<Symbol>,
}

#[derive(Clone, Copy)]
struct Section {
    kind: u32,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    entry_size: u64,
    alignment: u64,
    address: u64,
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn slice(&self, offset: u64, size: u64) -> Result<&'a [u8], String> {
        let end = offset.checked_add(size).ok_or("ELF range overflow")?;
        let start = usize::try_from(offset).map_err(|_| "ELF offset too large")?;
        let end = usize::try_from(end).map_err(|_| "ELF range too large")?;
        self.0
            .get(start..end)
            .ok_or_else(|| "ELF range outside input".into())
    }
    fn u16(&self, offset: u64) -> Result<u16, String> {
        let mut bytes = [0; 2];
        bytes.copy_from_slice(self.slice(offset, 2)?);
        Ok(u16::from_le_bytes(bytes))
    }
    fn u32(&self, offset: u64) -> Result<u32, String> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.slice(offset, 4)?);
        Ok(u32::from_le_bytes(bytes))
    }
    fn u64(&self, offset: u64) -> Result<u64, String> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.slice(offset, 8)?);
        Ok(u64::from_le_bytes(bytes))
    }
    fn string(&self, offset: u32) -> Result<String, String> {
        let tail = self
            .0
            .get(offset as usize..)
            .ok_or("ELF string offset outside table")?;
        let end = tail
            .iter()
            .position(|&byte| byte == 0)
            .ok_or("unterminated ELF string")?;
        let bytes = &tail[..end];
        if bytes.is_empty() || bytes.len() > 1024 || !bytes.iter().all(u8::is_ascii_graphic) {
            return Err("ELF names must be nonempty printable ASCII, at most 1024 bytes".into());
        }
        String::from_utf8(bytes.to_vec()).map_err(|_| "invalid ELF string".into())
    }
}

fn one(sections: &[Section], kind: u32) -> Result<Option<(usize, Section)>, String> {
    let mut found = sections
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, s)| s.kind == kind);
    let first = found.next();
    if found.next().is_some() {
        return Err(format!("multiple ELF sections of type {kind:#x}"));
    }
    Ok(first)
}

fn strings<'a>(r: &Reader<'a>, sections: &[Section], link: u32) -> Result<Reader<'a>, String> {
    let section = sections
        .get(link as usize)
        .ok_or("invalid ELF string-table link")?;
    if section.kind != STRTAB {
        return Err("ELF link does not identify a string table".into());
    }
    let data = r.slice(section.offset, section.size)?;
    if data.first() != Some(&0) {
        return Err("ELF string table lacks initial NUL".into());
    }
    Ok(Reader(data))
}

fn versions(
    r: &Reader<'_>,
    section: Section,
    names: &Reader<'_>,
) -> Result<BTreeMap<u16, String>, String> {
    let data = Reader(r.slice(section.offset, section.size)?);
    if section.info == 0 || section.info > 4096 || u64::from(section.info) > section.size / 20 {
        return Err("invalid ELF version-definition count".into());
    }
    let mut result = BTreeMap::new();
    let mut offset = 0;
    let mut name_bytes = 0;
    for index in 0..section.info {
        data.slice(offset, 20)?;
        if data.u16(offset)? != 1 {
            return Err("unsupported ELF version-definition revision".into());
        }
        let id = data.u16(offset + 4)?;
        let count = data.u16(offset + 6)?;
        let aux = u64::from(data.u32(offset + 12)?);
        let next = u64::from(data.u32(offset + 16)?);
        if id == 0 || id >= 0x8000 || count == 0 || count > 256 || aux < 20 {
            return Err("invalid ELF version definition".into());
        }
        let mut auxiliary = offset + aux;
        let mut version_name = None;
        for item in 0..count {
            data.slice(auxiliary, 8)?;
            let name = names.string(data.u32(auxiliary)?)?;
            name_bytes += name.len();
            if name_bytes > 1024 * 1024 {
                return Err("ELF version-name budget exceeded".into());
            }
            if item == 0 {
                version_name = Some(name);
            }
            let advance = u64::from(data.u32(auxiliary + 4)?);
            if item + 1 < count {
                if advance < 8 {
                    return Err("invalid ELF version auxiliary chain".into());
                }
                auxiliary += advance;
            } else if advance != 0 {
                return Err("unterminated ELF version auxiliary chain".into());
            }
        }
        let name = version_name.ok_or("missing ELF version name")?;
        if result.insert(id, name).is_some() {
            return Err("duplicate ELF version index".into());
        }
        if index + 1 < section.info {
            if next < 20 || offset + next < auxiliary + 8 {
                return Err("invalid ELF version-definition chain".into());
            }
            offset += next;
        } else if next != 0 {
            return Err("unterminated ELF version-definition chain".into());
        }
    }
    Ok(result)
}

impl Metadata {
    /// Reads and parses the ELF shared object at `path`.
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be read, is larger than [`MAX_FILE_BYTES`], or
    /// [`Metadata::parse`] rejects it; the message names the path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Parses an x86_64 ELF64 shared object held in memory. Every offset and size
    /// is bounds-checked, so malformed input fails instead of panicking.
    ///
    /// # Errors
    ///
    /// Fails if the input is larger than [`MAX_FILE_BYTES`], is not a
    /// little-endian x86_64 ELF64 shared object with a `DT_SONAME`, or has
    /// malformed dynamic-symbol, string or version tables.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("ELF exceeds 64 MiB input limit".into());
        }
        let r = Reader(bytes);
        let header = r.slice(0, 64)?;
        if &header[..7] != b"\x7fELF\x02\x01\x01" || !matches!(header[7], 0 | 3) || header[8] != 0 {
            return Err("expected ELF64 little-endian System V/Linux input".into());
        }
        if r.u16(16)? != 3 || r.u16(18)? != 62 || r.u32(20)? != 1 || r.u16(52)? != 64 {
            return Err("expected an x86_64 ELF shared object".into());
        }
        let offset = r.u64(40)?;
        let count = r.u16(60)?;
        if offset == 0 || count == 0 || r.u16(58)? != 64 {
            return Err(
                "ELF requires ordinary 64-byte section headers; extended/absent tables unsupported"
                    .into(),
            );
        }
        r.slice(offset, u64::from(count) * 64)?;
        let mut sections = Vec::new();
        for index in 0..count {
            let pos = offset + u64::from(index) * 64;
            sections.push(Section {
                kind: r.u32(pos + 4)?,
                offset: r.u64(pos + 24)?,
                size: r.u64(pos + 32)?,
                link: r.u32(pos + 40)?,
                info: r.u32(pos + 44)?,
                entry_size: r.u64(pos + 56)?,
                alignment: r.u64(pos + 48)?,
                address: r.u64(pos + 16)?,
            });
        }
        let (sym_index, sym_section) =
            one(&sections, DYNSYM)?.ok_or("missing ELF dynamic symbols")?;
        if sym_section.entry_size != 24 || sym_section.size == 0 || sym_section.size % 24 != 0 {
            return Err("invalid ELF dynamic-symbol entry size".into());
        }
        let names = strings(&r, &sections, sym_section.link)?;
        let symbols = Reader(r.slice(sym_section.offset, sym_section.size)?);
        if symbols.slice(0, 24)?.iter().any(|&byte| byte != 0) {
            return Err("ELF dynamic-symbol zero entry is not null".into());
        }
        let (_, dynamic) = one(&sections, DYNAMIC)?.ok_or("missing ELF dynamic section")?;
        if dynamic.entry_size != 16 || dynamic.size % 16 != 0 {
            return Err("invalid ELF dynamic entry size".into());
        }
        let dynamic_names = strings(&r, &sections, dynamic.link)?;
        let dynamic_data = Reader(r.slice(dynamic.offset, dynamic.size)?);
        let mut soname = None;
        let mut terminated = false;
        for pos in (0..dynamic.size).step_by(16) {
            let tag = dynamic_data.u64(pos)?;
            if tag == 0 {
                terminated = true;
                break;
            }
            if tag == 14 {
                let name = u32::try_from(dynamic_data.u64(pos + 8)?)
                    .map_err(|_| "ELF SONAME offset too large")?;
                if soname.replace(dynamic_names.string(name)?).is_some() {
                    return Err("duplicate ELF SONAME".into());
                }
            }
        }
        if !terminated {
            return Err("unterminated ELF dynamic section".into());
        }
        let soname = soname.ok_or("ELF shared object lacks SONAME")?;
        let symbol_count = sym_section.size / 24;
        if symbol_count > 100_000 {
            return Err("ELF dynamic-symbol count exceeds 100000".into());
        }
        let version_section = one(&sections, VERSYM)?;
        let version_symbols = if let Some((_, section)) = version_section {
            if section.link as usize != sym_index
                || section.entry_size != 2
                || section.size != symbol_count * 2
            {
                return Err("invalid ELF version-symbol table".into());
            }
            Some(Reader(r.slice(section.offset, section.size)?))
        } else {
            None
        };
        let version_names = if let Some((_, section)) = one(&sections, VERDEF)? {
            if version_symbols.is_none() || section.link != sym_section.link {
                return Err("invalid ELF version-definition link".into());
            }
            versions(&r, section, &names)?
        } else {
            BTreeMap::new()
        };
        let mut entries = Vec::new();
        let mut keys = BTreeSet::new();
        let mut defaults = BTreeSet::new();
        let mut name_bytes = 0;
        for index in 1..symbol_count {
            let pos = index * 24;
            let info = symbols.slice(pos + 4, 1)?[0];
            let binding = info >> 4;
            let section = symbols.u16(pos + 6)?;
            // Imported or local symbols are not definitions supplied by this DSO.
            if section == 0 || binding == 0 {
                continue;
            }
            if !matches!(binding, 1 | 2 | 10) {
                return Err(format!("unsupported ELF symbol binding {binding}"));
            }
            if section == 0xffff
                || (section as usize >= sections.len() && !matches!(section, 0xfff1 | 0xfff2))
            {
                return Err("unsupported or invalid ELF symbol section index".into());
            }
            let visibility = symbols.slice(pos + 5, 1)?[0];
            let section_alignment = sections
                .get(section as usize)
                .map_or(0, |section| section.alignment);
            if section_alignment != 0 && !section_alignment.is_power_of_two() {
                return Err("invalid ELF section alignment".into());
            }
            if visibility > 3 {
                return Err("unsupported ELF symbol visibility bits".into());
            }
            let name = names.string(symbols.u32(pos)?)?;
            let raw = match &version_symbols {
                Some(table) => table.u16(index * 2)?,
                None => 1,
            };
            let id = raw & 0x7fff;
            if section == 0xfff1 && version_names.values().any(|version| version == &name) {
                continue;
            }
            let (version, default_version) = if id > 1 {
                (
                    Some(
                        version_names
                            .get(&id)
                            .ok_or("undefined ELF version-definition index")?
                            .clone(),
                    ),
                    Some(raw & 0x8000 == 0),
                )
            } else {
                if raw & 0x8000 != 0 || id == 0 {
                    return Err("invalid global ELF symbol version index".into());
                }
                (None, None)
            };
            name_bytes += name.len() + version.as_ref().map_or(0, String::len);
            if name_bytes > 8 * 1024 * 1024 {
                return Err("ELF symbol-name budget exceeded".into());
            }
            if !keys.insert((name.clone(), version.clone(), default_version)) {
                return Err("duplicate ELF symbol definition".into());
            }
            if default_version == Some(true) && !defaults.insert(name.clone()) {
                return Err("multiple default ELF symbol versions".into());
            }
            entries.push(Symbol {
                name,
                version,
                default_version,
                symbol_type: info & 15,
                binding,
                visibility,
                section,
                section_alignment,
                value: symbols.u64(pos + 8)?,
                size: symbols.u64(pos + 16)?,
                same_range: None,
            });
        }
        entries.sort();
        let mut ranges = BTreeMap::<_, Vec<usize>>::new();
        for (index, symbol) in entries.iter().enumerate() {
            if symbol.section < 0xff00 && symbol.size != 0 {
                ranges
                    .entry((
                        symbol.section,
                        symbol.value,
                        symbol.symbol_type,
                        symbol.size,
                    ))
                    .or_default()
                    .push(index);
            }
        }
        let mut group = 0;
        for indexes in ranges.values().filter(|indexes| indexes.len() > 1) {
            group += 1;
            for &index in indexes {
                entries[index].same_range = Some(group);
            }
        }
        Ok(Self {
            soname,
            symbols: entries,
        })
    }

    /// Reads [`Metadata::to_tsv`] output back, one entry per SONAME in the order
    /// first seen. Several libraries' rows may share one table under one header.
    pub fn from_tsv(text: &str) -> Result<Vec<Self>, String> {
        let mut lines = text.lines();
        if lines.next() != Some(TSV_HEADER.trim_end()) || !text.ends_with('\n') {
            return Err("not an ELF metadata table".into());
        }
        let mut libraries: Vec<Self> = Vec::new();
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            let [soname, name, version, default, kind, binding, visibility, section, alignment, value, size, group] =
                fields[..]
            else {
                return Err(format!("malformed ELF metadata row {line:?}"));
            };
            let number = |text: &str| {
                text.parse::<u64>()
                    .map_err(|_| format!("invalid number {text:?} in {line:?}"))
            };
            let named = |text: &str, names: &[(&str, u8)]| {
                names
                    .iter()
                    .find(|(n, _)| *n == text)
                    .map(|&(_, v)| v)
                    .ok_or_else(|| format!("unsupported value {text:?} in {line:?}"))
            };
            let symbol = Symbol {
                name: name.into(),
                version: (version != "-").then(|| version.into()),
                default_version: match default {
                    "yes" => Some(true),
                    "no" => Some(false),
                    "-" => None,
                    _ => return Err(format!("invalid default {default:?}")),
                },
                symbol_type: named(
                    kind,
                    &[
                        ("NOTYPE", 0),
                        ("OBJECT", 1),
                        ("FUNC", 2),
                        ("TLS", 6),
                        ("IFUNC", 10),
                    ],
                )?,
                binding: named(binding, &[("GLOBAL", 1), ("WEAK", 2), ("GNU_UNIQUE", 10)])?,
                visibility: named(
                    visibility,
                    &[
                        ("DEFAULT", 0),
                        ("INTERNAL", 1),
                        ("HIDDEN", 2),
                        ("PROTECTED", 3),
                    ],
                )?,
                section: section
                    .parse()
                    .map_err(|_| format!("invalid section in {line:?}"))?,
                section_alignment: number(alignment)?,
                value: value
                    .strip_prefix("0x")
                    .and_then(|v| u64::from_str_radix(v, 16).ok())
                    .ok_or_else(|| format!("invalid value in {line:?}"))?,
                size: number(size)?,
                same_range: match group {
                    "-" => None,
                    g => Some(number(g)? as usize),
                },
            };
            if soname.is_empty() || name.is_empty() {
                return Err(format!("empty SONAME or name in {line:?}"));
            }
            match libraries.iter_mut().find(|m| m.soname == soname) {
                Some(library) => library.symbols.push(symbol),
                None => libraries.push(Self {
                    soname: soname.into(),
                    symbols: vec![symbol],
                }),
            }
        }
        Ok(libraries)
    }

    /// Stable tab-separated table of the symbols, one row per symbol under a
    /// header, which [`Metadata::from_tsv`] reads back.
    pub fn to_tsv(&self) -> String {
        let mut text = String::from(TSV_HEADER);
        for s in &self.symbols {
            let kind = match s.symbol_type {
                0 => "NOTYPE".into(),
                1 => "OBJECT".into(),
                2 => "FUNC".into(),
                6 => "TLS".into(),
                10 => "IFUNC".into(),
                n => format!("TYPE_{n}"),
            };
            let binding = match s.binding {
                1 => "GLOBAL",
                2 => "WEAK",
                10 => "GNU_UNIQUE",
                _ => "UNKNOWN",
            };
            let visibility = match s.visibility {
                0 => "DEFAULT",
                1 => "INTERNAL",
                2 => "HIDDEN",
                3 => "PROTECTED",
                _ => "UNKNOWN",
            };
            let default = match s.default_version {
                Some(true) => "yes",
                Some(false) => "no",
                None => "-",
            };
            let group = s
                .same_range
                .map_or_else(|| "-".into(), |value| value.to_string());
            writeln!(
                text,
                "{}\t{}\t{}\t{default}\t{kind}\t{binding}\t{visibility}\t{}\t{}\t0x{:x}\t{}\t{group}",
                self.soname,
                s.name,
                s.version.as_deref().unwrap_or("-"),
                s.section,
                s.section_alignment,
                s.value,
                s.size
            )
            .expect("writing to a String cannot fail");
        }
        text
    }
}

// Require section metadata to describe the tables the loader actually uses.
fn profile_tables(bytes: &[u8]) -> Result<(Section, Section, Vec<Section>), String> {
    let r = Reader(bytes);
    let shoff = r.u64(40)?;
    let shnum = r.u16(60)?;
    let mut sections = Vec::new();
    for i in 0..shnum {
        let p = shoff
            .checked_add(u64::from(i) * 64)
            .ok_or("section offset overflow")?;
        sections.push(Section {
            kind: r.u32(p + 4)?,
            address: r.u64(p + 16)?,
            offset: r.u64(p + 24)?,
            size: r.u64(p + 32)?,
            link: r.u32(p + 40)?,
            info: r.u32(p + 44)?,
            alignment: r.u64(p + 48)?,
            entry_size: r.u64(p + 56)?,
        });
    }
    let (_, sym) = one(&sections, DYNSYM)?.ok_or("missing dynamic symbols")?;
    let (_, dynamic) = one(&sections, DYNAMIC)?.ok_or("missing dynamic section")?;
    let (_, versym) = one(&sections, VERSYM)?.ok_or("clock profile requires version symbols")?;
    let (_, verdef) =
        one(&sections, VERDEF)?.ok_or("clock profile requires version definitions")?;
    if dynamic.link != sym.link {
        return Err("loader SONAME and symbols require one string table".into());
    }
    let names = *sections
        .get(sym.link as usize)
        .ok_or("invalid string link")?;
    let phoff = r.u64(32)?;
    let phnum = r.u16(56)?;
    if r.u16(54)? != 56 || phnum == 0 || phnum == 0xffff {
        return Err("clock profile requires ordinary program headers".into());
    }
    r.slice(phoff, u64::from(phnum) * 56)?;
    let mut loads = Vec::new();
    let mut dynamic_matches = 0;
    let mut dynamic_count = 0;
    for i in 0..phnum {
        let p = phoff
            .checked_add(u64::from(i) * 56)
            .ok_or("program offset overflow")?;
        let kind = r.u32(p)?;
        let offset = r.u64(p + 8)?;
        let address = r.u64(p + 16)?;
        let size = r.u64(p + 32)?;
        if kind == 1 {
            let memory_size = r.u64(p + 40)?;
            let alignment = r.u64(p + 48)?;
            if size > memory_size
                || address.checked_add(memory_size).is_none()
                || (alignment > 1
                    && (!alignment.is_power_of_two() || offset % alignment != address % alignment))
            {
                return Err("invalid loader segment extent or alignment".into());
            }
            r.slice(offset, size)?;
            loads.push((offset, address, size));
        }
        if kind == 2 {
            dynamic_count += 1;
        }
        if kind == 2
            && offset == dynamic.offset
            && address == dynamic.address
            && size == dynamic.size
        {
            dynamic_matches += 1;
        }
    }
    if dynamic_matches != 1 || dynamic_count != 1 {
        return Err("dynamic section does not match loader segment".into());
    }
    for section in [sym, names, versym, verdef, dynamic] {
        let mut mapped = false;
        for &(offset, address, size) in &loads {
            if let Some(delta) = section.address.checked_sub(address) {
                if delta <= size && section.size <= size - delta {
                    let at = offset.checked_add(delta).ok_or("load mapping overflow")?;
                    if at != section.offset {
                        return Err("conflicting loader table mapping".into());
                    }
                    mapped = true;
                }
            }
        }
        if !mapped {
            return Err("ELF table is not backed by its loader mapping".into());
        }
    }
    let mut tags = BTreeMap::new();
    for pos in (0..dynamic.size).step_by(16) {
        let p = dynamic
            .offset
            .checked_add(pos)
            .ok_or("dynamic offset overflow")?;
        let tag = r.u64(p)?;
        if tag == 0 {
            break;
        }
        if matches!(
            tag,
            5 | 6 | 10 | 11 | 0x6fff_fff0 | 0x6fff_fffc | 0x6fff_fffd
        ) && tags.insert(tag, r.u64(p + 8)?).is_some()
        {
            return Err("duplicate loader table tag".into());
        }
    }
    for (tag, expected) in [
        (5, names.address),
        (6, sym.address),
        (10, names.size),
        (11, 24),
        (0x6fff_fff0, versym.address),
        (0x6fff_fffc, verdef.address),
        (0x6fff_fffd, u64::from(verdef.info)),
    ] {
        if tags.get(&tag) != Some(&expected) {
            return Err("loader and section tables disagree".into());
        }
    }
    let end = versym
        .offset
        .checked_add(versym.size)
        .ok_or("version range overflow")?;
    let overlaps = |offset: u64, size: u64| -> Result<bool, String> {
        Ok(size != 0
            && versym.offset < offset.checked_add(size).ok_or("table range overflow")?
            && offset < end)
    };
    if overlaps(0, 64)?
        || overlaps(shoff, u64::from(shnum) * 64)?
        || overlaps(phoff, u64::from(phnum) * 56)?
    {
        return Err("version symbols overlap ELF headers".into());
    }
    for section in &sections {
        if section.kind != VERSYM && section.kind != 8 && overlaps(section.offset, section.size)? {
            return Err("version symbols overlap another section".into());
        }
    }
    Ok((sym, versym, sections))
}

pub(crate) fn libc_clock_copy(libc: &[u8], rt: &[u8]) -> Result<Option<Vec<u8>>, String> {
    const CLOCKS: [&str; 5] = [
        "clock_gettime",
        "clock_getres",
        "clock_settime",
        "clock_nanosleep",
        "clock_getcpuclockid",
    ];
    let c = Metadata::parse(libc)?;
    let original = Metadata::parse(rt)?;
    if c.soname != "libc.so.6" || original.soname != "librt.so.1" {
        return Err("clock profile requires libc.so.6 and librt.so.1 ELF inputs".into());
    }
    profile_tables(libc)?;
    for name in CLOCKS {
        let defaults: Vec<_> = c
            .symbols
            .iter()
            .filter(|s| s.name == name && s.default_version == Some(true))
            .collect();
        if defaults.len() != 1
            || defaults[0].version.as_deref() != Some("GLIBC_2.17")
            || !matches!(defaults[0].symbol_type, 2 | 10)
            || !matches!(defaults[0].binding, 1 | 2)
            || !matches!(defaults[0].visibility, 0 | 3)
        {
            return Err(format!("clock profile requires libc {name}@@GLIBC_2.17"));
        }
    }
    let mut expected = original.clone();
    let mut changed = BTreeSet::new();
    for symbol in &mut expected.symbols {
        if CLOCKS.contains(&symbol.name.as_str()) {
            if !matches!(symbol.symbol_type, 2 | 10)
                || !matches!(symbol.binding, 1 | 2)
                || !matches!(symbol.visibility, 0 | 3)
                || symbol.default_version.is_none()
            {
                return Err(format!(
                    "unsupported librt clock definition: {}",
                    symbol.name
                ));
            }
            if symbol.default_version == Some(true) {
                if symbol.version.as_deref() != Some("GLIBC_2.2.5") {
                    return Err("unsupported librt default clock version".into());
                }
                changed.insert(symbol.name.clone());
                symbol.default_version = Some(false);
            }
        }
    }
    // Even no-op input must be consistent with its loader tables.
    let (sym, versym, sections) = profile_tables(rt)?;
    if changed.is_empty() {
        return Ok(None);
    }
    let r = Reader(rt);
    let names = strings(&r, &sections, sym.link)?;
    let mut derived = rt.to_vec();
    let mut patched = BTreeSet::new();
    for index in 1..sym.size / 24 {
        let pos = sym
            .offset
            .checked_add(index * 24)
            .ok_or("symbol offset overflow")?;
        if r.u16(pos + 6)? == 0 || r.slice(pos + 4, 1)?[0] >> 4 == 0 {
            continue;
        }
        let name = names.string(r.u32(pos)?)?;
        let at = versym
            .offset
            .checked_add(index * 2)
            .ok_or("version offset overflow")?;
        let version = r.u16(at)?;
        if changed.contains(&name) && version & 0x8000 == 0 {
            let byte = usize::try_from(at.checked_add(1).ok_or("version offset overflow")?)
                .map_err(|_| "version offset too large")?;
            let target = derived.get_mut(byte).ok_or("version offset outside file")?;
            *target |= 0x80;
            patched.insert(name);
        }
    }
    if patched != changed || Metadata::parse(&derived)? != expected {
        return Err("derived clock metadata differs beyond the permitted default flags".into());
    }
    Ok(Some(derived))
}
