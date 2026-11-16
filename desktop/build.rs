use std::fs;
use std::path::PathBuf;

fn le_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn le_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn push_res_entry(out: &mut Vec<u8>, res_type: u16, res_id: u16, data: &[u8]) {
    let data_size = data.len() as u32;
    let header_size: u32 = 32;
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    // Type: 0xFFFF + integer ID
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_type.to_le_bytes());
    // Name: 0xFFFF + integer ID
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_id.to_le_bytes());
    // DataVersion, MemoryFlags (MOVEABLE|PURE|DISCARDABLE), LanguageId (0x0409), Version, Characteristics
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0x1030u16.to_le_bytes());
    out.extend_from_slice(&0x0409u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn build_icon_res(ico: &[u8]) -> Vec<u8> {
    assert!(ico.len() >= 6, "Invalid ICO header");
    assert_eq!(le_u16(ico, 0), 0, "ICO reserved must be 0");
    assert_eq!(le_u16(ico, 2), 1, "ICO type must be 1");
    let count = le_u16(ico, 4) as usize;
    assert!(count > 0 && ico.len() >= 6 + count * 16, "Truncated ICO directory");

    let mut res = Vec::with_capacity(ico.len() + 256);
    // Standard 32-byte empty Win32 .res header
    res.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
        0xff, 0xff, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);

    let mut group_icon = Vec::with_capacity(6 + count * 14);
    group_icon.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    group_icon.extend_from_slice(&1u16.to_le_bytes()); // idType = 1 (ICON)
    group_icon.extend_from_slice(&(count as u16).to_le_bytes());

    for i in 0..count {
        let dir_off = 6 + i * 16;
        let width = ico[dir_off];
        let height = ico[dir_off + 1];
        let color_count = ico[dir_off + 2];
        let reserved = ico[dir_off + 3];
        let planes = le_u16(ico, dir_off + 4).max(1);
        let bit_count = le_u16(ico, dir_off + 6).max(32);
        let bytes_in_res = le_u32(ico, dir_off + 8) as usize;
        let img_offset = le_u32(ico, dir_off + 12) as usize;
        let icon_id = (i + 1) as u16;

        let img = &ico[img_offset..img_offset + bytes_in_res];
        push_res_entry(&mut res, 3 /* RT_ICON */, icon_id, img);

        group_icon.push(width);
        group_icon.push(height);
        group_icon.push(color_count);
        group_icon.push(reserved);
        group_icon.extend_from_slice(&planes.to_le_bytes());
        group_icon.extend_from_slice(&bit_count.to_le_bytes());
        group_icon.extend_from_slice(&(bytes_in_res as u32).to_le_bytes());
        group_icon.extend_from_slice(&icon_id.to_le_bytes());
    }

    push_res_entry(&mut res, 14 /* RT_GROUP_ICON */, 1, &group_icon);
    res
}

fn main() {
    let ico_path = PathBuf::from("../assets/icons/icon.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env == "msvc" {
        let ico = fs::read(&ico_path).expect("failed to read ../assets/icons/icon.ico");
        let res = build_icon_res(&ico);
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR missing"));
        let res_path = out_dir.join("eqntide_icon.res");
        fs::write(&res_path, &res).expect("failed to write eqntide_icon.res");
        println!("cargo:rustc-link-arg={}", res_path.display());
    } else {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../assets/icons/icon.ico");
        res.compile().expect("failed to compile Windows icon resource");
    }
}
