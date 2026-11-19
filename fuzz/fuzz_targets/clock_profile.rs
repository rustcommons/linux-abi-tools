#![no_main]
use libfuzzer_sys::fuzz_target;
const C: &[u8] = include_bytes!("../../tests/fixtures/clock-profile/c.so");
const RT: &[u8] = include_bytes!("../../tests/fixtures/clock-profile/rt.so");
fuzz_target!(|bytes: &[u8]| {
    let Some((&selector, data)) = bytes.split_first() else {
        return;
    };
    let mut c = C.to_vec();
    let mut rt = RT.to_vec();
    if selector & 2 == 0 {
        if selector & 1 == 0 {
            c = data.to_vec();
        } else {
            rt = data.to_vec();
        }
    } else if let Some(prefix) = data.get(..4) {
        let target = if selector & 1 == 0 { &mut c } else { &mut rt };
        let offset = u32::from_le_bytes(prefix.try_into().unwrap()) as usize % target.len();
        let length = (target.len() - offset).min(data.len() - 4);
        target[offset..offset + length].copy_from_slice(&data[4..4 + length]);
    }
    if let Ok(Some(copy)) = linux_abi_tools_fuzz::clock_copy(&c, &rt) {
        assert_eq!(copy.len(), rt.len());
        for (&before, &after) in rt.iter().zip(&copy) {
            assert!(before == after || (before ^ after == 0x80 && after & 0x80 != 0));
        }
    }
});
