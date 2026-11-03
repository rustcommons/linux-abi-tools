//! Bounded validation of the exact generated objects and GNU archive inventory.
use super::inputs::hash;
use std::collections::{BTreeMap, BTreeSet};

fn slice(b: &[u8], off: usize, size: usize) -> Result<&[u8], String> {
    b.get(off..off.checked_add(size).ok_or("ELF range overflow")?)
        .ok_or_else(|| "ELF range outside input".into())
}
fn u16at(b: &[u8], p: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        slice(b, p, 2)?.try_into().map_err(|_| "ELF u16")?,
    ))
}
fn u32at(b: &[u8], p: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        slice(b, p, 4)?.try_into().map_err(|_| "ELF u32")?,
    ))
}
fn u64at(b: &[u8], p: usize) -> Result<usize, String> {
    usize::try_from(u64::from_le_bytes(
        slice(b, p, 8)?.try_into().map_err(|_| "ELF u64")?,
    ))
    .map_err(|_| "ELF offset too large".into())
}
fn string(b: &[u8], p: usize) -> Result<&str, String> {
    let b = b.get(p..).ok_or("symbol string offset")?;
    let n = b
        .iter()
        .position(|&c| c == 0)
        .ok_or("unterminated symbol string")?;
    std::str::from_utf8(&b[..n]).map_err(|e| e.to_string())
}
#[derive(Default)]
pub(super) struct Symbols {
    pub defined: BTreeMap<String, u8>,
    pub undefined: BTreeSet<String>,
}
pub(super) fn object(b: &[u8]) -> Result<Symbols, String> {
    let h = slice(b, 0, 64)?;
    if &h[..7] != b"\x7fELF\x02\x01\x01"
        || !matches!(h[7], 0 | 3)
        || h[8] != 0
        || u16at(b, 16)? != 1
        || u16at(b, 18)? != 62
        || u32at(b, 20)? != 1
        || u16at(b, 52)? != 64
        || u16at(b, 56)? != 0
        || u16at(b, 58)? != 64
    {
        return Err("expected x86_64 ELF64 relocatable object".into());
    }
    let off = u64at(b, 40)?;
    let count = usize::from(u16at(b, 60)?);
    if off < 64 || count == 0 || count > 10000 {
        return Err("unsupported ELF section table".into());
    }
    let table = slice(b, off, count * 64)?;
    let mut result = Symbols::default();
    let mut symtabs = 0;
    for sec in table.as_chunks::<64>().0 {
        let kind = u32at(sec, 4)?;
        let pos = u64at(sec, 24)?;
        let size = u64at(sec, 32)?;
        if kind != 8 {
            slice(b, pos, size)?;
        }
        if kind != 2 {
            continue;
        }
        symtabs += 1;
        if symtabs > 1 || u64at(sec, 56)? != 24 || size < 24 || size % 24 != 0 || size / 24 > 100000
        {
            return Err("invalid ELF symbol table".into());
        }
        let link = usize::try_from(u32at(sec, 40)?).map_err(|_| "symbol link")?;
        let names = slice(table, link.checked_mul(64).ok_or("section overflow")?, 64)?;
        if u32at(names, 4)? != 3 {
            return Err("symbol names not string table".into());
        }
        let names = slice(b, u64at(names, 24)?, u64at(names, 32)?)?;
        if slice(b, pos, 24)?.iter().any(|&c| c != 0) {
            return Err("ELF null symbol is not zero".into());
        }
        for sym in slice(b, pos, size)?.as_chunks::<24>().0 {
            let bind = sym[4] >> 4;
            if bind == 0 {
                continue;
            }
            if !matches!(bind, 1 | 2) || sym[5] > 3 {
                return Err("unsupported symbol binding/visibility".into());
            }
            let section = u16at(sym, 6)?;
            if section == 0xffff
                || (usize::from(section) >= count && !matches!(section, 0xfff1 | 0xfff2))
            {
                return Err("invalid symbol section".into());
            }
            let name = string(names, u32at(sym, 0)? as usize)?.to_string();
            if name.is_empty() || name.len() > 4096 {
                return Err("symbol name too long".into());
            }
            if section == 0 {
                result.undefined.insert(name);
            } else {
                if result.defined.insert(name, sym[5]).is_some() {
                    return Err("duplicate ELF global definition".into());
                }
            }
        }
    }
    // x86_64 RELA entries only: audit ranges, symbol/target links and recipe's
    // observed PIC relocation forms. This does not prove final linked PIC or closure.
    let mut relocations = 0usize;
    for sec in table.as_chunks::<64>().0 {
        let kind = u32at(sec, 4)?;
        if kind == 9 {
            return Err("unsupported x86_64 REL relocation section".into());
        }
        if kind != 4 {
            continue;
        }
        let size = u64at(sec, 32)?;
        if u64at(sec, 56)? != 24 || size % 24 != 0 {
            return Err("invalid RELA entry size".into());
        }
        relocations = relocations
            .checked_add(size / 24)
            .ok_or("relocation count overflow")?;
        if relocations > 1000000 {
            return Err("relocation budget exceeded".into());
        }
        let sym = slice(
            table,
            (u32at(sec, 40)? as usize)
                .checked_mul(64)
                .ok_or("relocation symbol link")?,
            64,
        )?;
        let target = slice(
            table,
            (u32at(sec, 44)? as usize)
                .checked_mul(64)
                .ok_or("relocation target link")?,
            64,
        )?;
        if u32at(sym, 4)? != 2 {
            return Err("relocation link is not symbol table".into());
        }
        let target_size = u64at(target, 32)?;
        let executable = u64at(target, 8)? & 4 != 0;
        for rel in slice(b, u64at(sec, 24)?, size)?.as_chunks::<24>().0 {
            let info = u64at(rel, 8)?;
            let ty = (info & 0xffff_ffff) as u32;
            if !matches!(ty, 1 | 2 | 4 | 9 | 41 | 42) || (ty == 1 && executable) {
                return Err("unsupported/non-PIC executable relocation".into());
            }
            if (info >> 32) >= u64at(sym, 32)? / 24 {
                return Err("relocation symbol outside table".into());
            }
            let width = if ty == 1 { 8 } else { 4 };
            if u64at(rel, 0)?
                .checked_add(width)
                .is_none_or(|end| end > target_size)
            {
                return Err("relocation outside target section".into());
            }
        }
    }
    if symtabs != 1 {
        return Err("object lacks symbol table".into());
    }
    Ok(result)
}

pub(super) fn validate(
    bytes: &[u8],
    objects: &BTreeMap<String, Vec<u8>>,
    hidden: bool,
) -> Result<Vec<String>, String> {
    if bytes.len() > 32 * 1024 * 1024 || !bytes.starts_with(b"!<arch>\n") {
        return Err("expected bounded ordinary GNU archive, not thin archive".into());
    }
    let mut pos = 8;
    let mut long = &b""[..];
    let mut seen = BTreeSet::new();
    let mut syms = BTreeMap::new();
    let mut defined = BTreeMap::new();
    let mut undefined = BTreeSet::new();
    let mut index = None;
    while pos < bytes.len() {
        let member_offset = pos;
        let h = slice(bytes, pos, 60)?;
        if &h[58..] != b"`\n" {
            return Err("invalid ar header".into());
        }
        let name = std::str::from_utf8(&h[..16])
            .map_err(|e| e.to_string())?
            .trim();
        let n = std::str::from_utf8(&h[48..58])
            .map_err(|e| e.to_string())?
            .trim()
            .parse::<usize>()
            .map_err(|_| "invalid ar size")?;
        pos += 60;
        let data = slice(bytes, pos, n)?;
        pos = pos.checked_add(n).ok_or("archive size overflow")?;
        if pos % 2 != 0 {
            if slice(bytes, pos, 1)? != b"\n" {
                return Err("invalid ar padding".into());
            }
            pos += 1;
        }
        if name == "/" {
            if index.replace(data).is_some() {
                return Err("duplicate ar symbol index".into());
            }
            continue;
        }
        if name == "//" {
            if !long.is_empty() {
                return Err("duplicate ar names".into());
            }
            long = data;
            continue;
        }
        let name = if let Some(offset) = name.strip_prefix('/') {
            let offset = offset.parse::<usize>().map_err(|_| "unsupported ar name")?;
            let tail = long.get(offset..).ok_or("ar name outside table")?;
            let end = tail
                .windows(2)
                .position(|w| w == b"/\n")
                .ok_or("unterminated ar name")?;
            std::str::from_utf8(&tail[..end]).map_err(|e| e.to_string())?
        } else {
            name.strip_suffix('/').ok_or("unsupported ar member name")?
        };
        let expected = objects.get(name).ok_or("unexpected ar member")?;
        if !seen.insert(name.to_string()) || data != expected {
            return Err("archive member differs from validated object".into());
        }
        let symbols = object(data)?;
        for (n, v) in &symbols.defined {
            defined.insert(n.clone(), *v);
        }
        undefined.extend(symbols.undefined);
        syms.insert(member_offset, symbols.defined);
    }
    if seen.len() != objects.len() {
        return Err("archive member inventory differs".into());
    }
    let index = index.ok_or("archive lacks GNU symbol index")?;
    let n = u32::from_be_bytes(slice(index, 0, 4)?.try_into().map_err(|_| "ar index")?) as usize;
    if n > 100000 {
        return Err("ar symbol index too large".into());
    }
    let mut names = 4 + n * 4;
    slice(index, 4, n * 4)?;
    let mut indexed = BTreeSet::new();
    for i in 0..n {
        let off = u32::from_be_bytes(
            slice(index, 4 + i * 4, 4)?
                .try_into()
                .map_err(|_| "ar index")?,
        ) as usize;
        let name = string(index, names)?;
        names = names
            .checked_add(name.len() + 1)
            .ok_or("ar string overflow")?;
        if !indexed.insert((off, name.to_string()))
            || !syms.get(&off).is_some_and(|s| s.contains_key(name))
        {
            return Err("ar symbol index targets wrong member".into());
        }
    }
    let expected_index: BTreeSet<_> = syms
        .iter()
        .flat_map(|(off, defs)| defs.keys().map(move |name| (*off, name.clone())))
        .collect();
    if indexed != expected_index {
        return Err("ar symbol index incomplete".into());
    }
    if names != index.len() && !(names + 1 == index.len() && index[names] == 0) {
        return Err("ar symbol index trailing bytes".into());
    }
    for name in [
        "_Unwind_RaiseException",
        "_Unwind_Resume",
        "_Unwind_DeleteException",
        "_Unwind_ForcedUnwind",
        "_Unwind_Backtrace",
        "_Unwind_GetIP",
        "_Unwind_GetLanguageSpecificData",
    ] {
        if defined.get(name) != Some(&(if hidden { 2 } else { 0 })) {
            return Err(format!(
                "missing/wrong visibility runtime definition {name}"
            ));
        }
    }
    let remaining: Vec<_> = undefined
        .into_iter()
        .filter(|s| !defined.contains_key(s))
        .collect();
    if remaining
        .iter()
        .any(|s| s.starts_with("_Z") || s.starts_with("__cxa") || s == "__gcc_personality_v0")
    {
        return Err("unexpected C++/GCC runtime dependency".into());
    }
    Ok(remaining)
}

pub(super) fn inventory(objects: &BTreeMap<String, Vec<u8>>) -> String {
    objects
        .iter()
        .map(|(n, b)| format!("{}\t{}\n", hash(b), n))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    const NAMES: [&str; 7] = [
        "_Unwind_RaiseException",
        "_Unwind_Resume",
        "_Unwind_DeleteException",
        "_Unwind_ForcedUnwind",
        "_Unwind_Backtrace",
        "_Unwind_GetIP",
        "_Unwind_GetLanguageSpecificData",
    ];
    fn fixture(hidden: bool) -> Vec<u8> {
        let mut strings = vec![0];
        let mut symbols = vec![0; 24];
        for name in NAMES {
            let off = strings.len() as u32;
            strings.extend(name.as_bytes());
            strings.push(0);
            let mut sym = [0; 24];
            sym[..4].copy_from_slice(&off.to_le_bytes());
            sym[4] = 0x12;
            sym[5] = if hidden { 2 } else { 0 };
            sym[6..8].copy_from_slice(&1u16.to_le_bytes());
            symbols.extend(sym);
        }
        let mut b = vec![0; 64];
        b[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        b[16..18].copy_from_slice(&1u16.to_le_bytes());
        b[18..20].copy_from_slice(&62u16.to_le_bytes());
        b[20..24].copy_from_slice(&1u32.to_le_bytes());
        b[52..54].copy_from_slice(&64u16.to_le_bytes());
        b[58..60].copy_from_slice(&64u16.to_le_bytes());
        b[60..62].copy_from_slice(&4u16.to_le_bytes());
        b.push(0xc3);
        let stringoff = b.len();
        b.extend(&strings);
        let symoff = b.len();
        b.extend(&symbols);
        let shoff = b.len();
        b[40..48].copy_from_slice(&(shoff as u64).to_le_bytes());
        b.resize(shoff + 4 * 64, 0);
        for (i, kind, off, size, link, entry) in [
            (1, 1, 64, 1, 0, 0),
            (2, 3, stringoff, strings.len(), 0, 0),
            (3, 2, symoff, symbols.len(), 2, 24),
        ] {
            let p = shoff + i * 64;
            b[p + 4..p + 8].copy_from_slice(&(kind as u32).to_le_bytes());
            b[p + 24..p + 32].copy_from_slice(&(off as u64).to_le_bytes());
            b[p + 32..p + 40].copy_from_slice(&(size as u64).to_le_bytes());
            b[p + 40..p + 44].copy_from_slice(&(link as u32).to_le_bytes());
            b[p + 56..p + 64].copy_from_slice(&(entry as u64).to_le_bytes());
        }
        b
    }
    fn member(b: &mut Vec<u8>, name: &str, data: &[u8]) {
        b.extend(
            format!(
                "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                0,
                0,
                0,
                "644",
                data.len()
            )
            .as_bytes(),
        );
        b.extend(data);
        if !b.len().is_multiple_of(2) {
            b.push(b'\n');
        }
    }
    fn archive(object: &[u8]) -> Vec<u8> {
        let mut idx = Vec::new();
        idx.extend((NAMES.len() as u32).to_be_bytes());
        let size = 4 + 4 * NAMES.len() + NAMES.iter().map(|n| n.len() + 1).sum::<usize>();
        let off = 8 + 60 + size + size % 2;
        for _ in NAMES {
            idx.extend((off as u32).to_be_bytes());
        }
        for n in NAMES {
            idx.extend(n.as_bytes());
            idx.push(0);
        }
        let mut b = b"!<arch>\n".to_vec();
        member(&mut b, "/", &idx);
        member(&mut b, "runtime.o/", object);
        b
    }
    #[test]
    fn target_visibility_inventory_and_index() {
        let object = fixture(true);
        let a = archive(&object);
        let objects = BTreeMap::from([("runtime.o".into(), object.clone())]);
        assert!(validate(&a, &objects, true).is_ok());
        assert!(validate(&a, &objects, false).is_err());
        let mut bad = a.clone();
        bad[72] = 0xff;
        assert!(validate(&bad, &objects, true).is_err());
        let mut badobj = object;
        badobj[18] = 183;
        assert!(self::object(&badobj).is_err());
    }
    #[test]
    fn all_truncations_refuse_without_panics() {
        let object = fixture(false);
        let a = archive(&object);
        let objects = BTreeMap::from([("runtime.o".into(), object)]);
        for end in 0..a.len() {
            assert!(validate(&a[..end], &objects, false).is_err(), "{end}");
        }
    }
    #[test]
    fn thin_wrong_member_and_outside_sections_refuse() {
        let mut object = fixture(false);
        let a = archive(&object);
        assert!(validate(b"!<thin>\n", &BTreeMap::new(), false).is_err());
        assert!(validate(
            &a,
            &BTreeMap::from([("wrong.o".into(), object.clone())]),
            false
        )
        .is_err());
        object[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(self::object(&object).is_err());
    }
    #[test]
    fn relocation_links_ranges_and_absolute_text_refuse() {
        let mut b = fixture(true);
        let sh = u64at(&b, 40).unwrap();
        b[60..62].copy_from_slice(&5u16.to_le_bytes());
        b.resize(sh + 5 * 64, 0);
        let off = b.len();
        b.resize(off + 24, 0);
        let p = sh + 4 * 64;
        b[p + 4..p + 8].copy_from_slice(&4u32.to_le_bytes());
        b[p + 24..p + 32].copy_from_slice(&(off as u64).to_le_bytes());
        b[p + 32..p + 40].copy_from_slice(&24u64.to_le_bytes());
        b[p + 40..p + 44].copy_from_slice(&3u32.to_le_bytes());
        b[p + 44..p + 48].copy_from_slice(&1u32.to_le_bytes());
        b[p + 56..p + 64].copy_from_slice(&24u64.to_le_bytes());
        b[sh + 64 + 8..sh + 64 + 16].copy_from_slice(&4u64.to_le_bytes());
        b[sh + 64 + 32..sh + 64 + 40].copy_from_slice(&8u64.to_le_bytes());
        b[off + 8..off + 16].copy_from_slice(&((1u64 << 32) | 2).to_le_bytes());
        assert!(object(&b).is_ok());
        for ty in [1u64, 999, (999 << 32) | 2] {
            let mut bad = b.clone();
            bad[off + 8..off + 16].copy_from_slice(&ty.to_le_bytes());
            assert!(object(&bad).is_err());
        }
        let mut bad = b.clone();
        bad[off..off + 8].copy_from_slice(&9u64.to_le_bytes());
        assert!(object(&bad).is_err());
        b[p + 40..p + 44].copy_from_slice(&99u32.to_le_bytes());
        assert!(object(&b).is_err());
    }
}
