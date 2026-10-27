use std::fs::File;
use std::io::Write;

fn main() {
    // فقط برای بیلد ویندوز آیکون ساخته و متصل می‌شود
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let icon_path = "icon.ico";
        let ico_bytes = create_mds_icon_bytes();
        let mut file = File::create(icon_path).expect("Failed to create icon file");
        file.write_all(&ico_bytes).expect("Failed to write icon bytes");

        let mut res = winres::WindowsResource::new();
        res.set_icon(icon_path);
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resource: {}", e);
        }
    }
}

// ساخت استاندارد فایل ICO ویندوز به صورت بایت به بایت
fn create_mds_icon_bytes() -> Vec<u8> {
    const W: u32 = 48;
    const H: u32 = 48;
    let mut rgba = vec![0u8; (W * H * 4) as usize];

    let font_m = [
        [1, 0, 0, 0, 1],
        [1, 1, 0, 1, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ];
    let font_d = [
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
    ];
    let font_s = [
        [0, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [0, 1, 1, 1, 0],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
    ];

    for y in 0..H {
        for x in 0..W {
            let idx = ((y * W + x) * 4) as usize;
            let dx = (x as i32 - 24).abs();
            let dy = (y as i32 - 24).abs();
            if dx > 22 && dy > 22 {
                continue;
            }
            rgba[idx] = 30;
            rgba[idx + 1] = 41;
            rgba[idx + 2] = 59;
            rgba[idx + 3] = 255;
        }
    }

    let mut draw_glyph = |glyph: &[[u8; 5]; 7], offset_x: usize, offset_y: usize, r: u8, g: u8, b: u8| {
        for gy in 0..7 {
            for gx in 0..5 {
                if glyph[gy][gx] == 1 {
                    for sy in 0..2 {
                        for sx in 0..2 {
                            let px = offset_x + gx * 2 + sx;
                            let py = offset_y + gy * 2 + sy;
                            if px < W as usize && py < H as usize {
                                let idx = (py * W as usize + px) * 4;
                                rgba[idx] = r;
                                rgba[idx + 1] = g;
                                rgba[idx + 2] = b;
                                rgba[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    };

    let top_y = 17;
    draw_glyph(&font_m, 6, top_y, 56, 189, 248);   // M (Cyan)
    draw_glyph(&font_d, 19, top_y, 255, 255, 255); // D (White)
    draw_glyph(&font_s, 32, top_y, 56, 189, 248);  // S (Cyan)

    let mut ico = Vec::new();

    // 1. هدر فایل (ICONDIR)
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());

    let and_row_bytes = ((W + 31) / 32) * 4;
    let and_mask_size = and_row_bytes * H;
    let xor_size = W * H * 4;
    let dib_header_size = 40u32;
    let bytes_in_res = dib_header_size + xor_size + and_mask_size;

    // 2. مشخصات تصویر (ICONDIRENTRY)
    ico.push(W as u8);
    ico.push(H as u8);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&bytes_in_res.to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());

    // 3. هدر تصویر (BITMAPINFOHEADER)
    ico.extend_from_slice(&40u32.to_le_bytes());
    ico.extend_from_slice(&(W as i32).to_le_bytes());
    ico.extend_from_slice(&((H * 2) as i32).to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&(xor_size + and_mask_size).to_le_bytes());
    ico.extend_from_slice(&0i32.to_le_bytes());
    ico.extend_from_slice(&0i32.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());

    // 4. داده‌های پیکسل (BGRA) از پایین به بالا
    for y in (0..H).rev() {
        for x in 0..W {
            let idx = ((y * W + x) * 4) as usize;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            ico.push(b);
            ico.push(g);
            ico.push(r);
            ico.push(a);
        }
    }

    // 5. ماسک شفافیت AND
    for y in (0..H).rev() {
        let mut row_bytes = vec![0u8; and_row_bytes as usize];
        for x in 0..W {
            let idx = ((y * W + x) * 4 + 3) as usize;
            let a = rgba[idx];
            if a == 0 {
                let byte_idx = (x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                row_bytes[byte_idx] |= 1 << bit_idx;
            }
        }
        ico.extend_from_slice(&row_bytes);
    }

    ico
}