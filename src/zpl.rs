//! Minimal ZPL II renderer to a grayscale bitmap (203 dpi).
//! Supported: ^XA ^XZ ^PW ^LL ^FO ^FT(as FO) ^CF ^A0/^A ^FD ^FS ^GB ^BQ (QR). Unknown commands are ignored.
use image::{GrayImage, Luma};
use qrcode::{Color, QrCode};

struct State {
    x: i32,
    y: i32,
    font_h: i32,
    pending_qr: Option<i32>, // magnification
    gb: Option<(i32, i32, i32)>,
    text: bool,
}

fn num(s: &str, i: usize, d: i32) -> i32 {
    s.split(',')
        .nth(i)
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(d)
}

/// Renders the first label in `src`. Returns an image of PW x LL dots.
pub fn render(src: &str) -> GrayImage {
    let (mut w, mut h) = (384i32, 240i32);
    let mut cmds: Vec<(String, String)> = vec![];
    for part in src.split('^').skip(1) {
        let (c, a) = part.split_at(part.len().min(2));
        cmds.push((
            c.to_uppercase(),
            a.trim_end_matches(['\n', '\r']).to_string(),
        ));
    }
    for (c, a) in &cmds {
        match c.as_str() {
            "PW" => w = num(a, 0, w).clamp(8, 384),
            "LL" => h = num(a, 0, h).clamp(8, 4000),
            _ => {}
        }
    }
    let mut img = GrayImage::from_pixel(w as u32, h as u32, Luma([255]));
    let mut s = State {
        x: 0,
        y: 0,
        font_h: 30,
        pending_qr: None,
        gb: None,
        text: false,
    };
    for (c, a) in cmds {
        match c.as_str() {
            "FO" | "FT" => {
                s.x = num(&a, 0, 0);
                s.y = num(&a, 1, 0);
            }
            "CF" => {
                s.font_h = num(
                    a.trim_start_matches(|ch: char| ch.is_ascii_alphabetic()),
                    0,
                    30,
                )
                .max(8)
            }
            c if c.starts_with('A') => {
                // ^A0N,h,w  (command is 2 chars: "A0"); handled above, else height in args
                s.font_h = num(&a, 1, s.font_h).max(8);
            }
            "GB" => s.gb = Some((num(&a, 0, 1), num(&a, 1, 1), num(&a, 2, 1))),
            "BQ" => s.pending_qr = Some(num(&a, 2, 4).clamp(1, 10)),
            "FD" => {
                let data = a.as_str();
                if let Some(m) = s.pending_qr.take() {
                    // ^FDQA,data or ^FDLA,data
                    let payload = data.split_once(',').map(|x| x.1).unwrap_or(data);
                    draw_qr(&mut img, payload, s.x, s.y, m);
                } else {
                    draw_text(&mut img, data, s.x, s.y, s.font_h);
                }
                s.text = true;
            }
            "FS" => {
                if let Some((bw, bh, t)) = s.gb.take() {
                    draw_box(&mut img, s.x, s.y, bw, bh, t);
                }
            }
            _ => {}
        }
        // ^GB is drawn at ^FS; fine for typical use.
    }
    img
}

/// Size in dots of text drawn by the built-in font at `height`.
pub fn text_extent(text: &str, height: i32) -> (i32, i32) {
    let scale = (height / 8).max(1);
    (text.chars().count() as i32 * 8 * scale, 8 * scale)
}

/// Side in dots of a QR code for `data` with module size `m`.
pub fn qr_size(data: &str, m: i32) -> i32 {
    QrCode::new(data.as_bytes())
        .map(|c| c.width() as i32)
        .unwrap_or(21)
        * m
}

fn put(img: &mut GrayImage, x: i32, y: i32) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, Luma([0]));
    }
}

fn draw_box(img: &mut GrayImage, x: i32, y: i32, w: i32, h: i32, t: i32) {
    for dy in 0..h.max(1) {
        for dx in 0..w.max(1) {
            if dx < t || dy < t || dx >= w - t || dy >= h - t {
                put(img, x + dx, y + dy);
            }
        }
    }
}

fn draw_text(img: &mut GrayImage, text: &str, x: i32, y: i32, height: i32) {
    use font8x8::UnicodeFonts;
    let scale = (height / 8).max(1);
    for (i, ch) in text.chars().enumerate() {
        let g = font8x8::BASIC_FONTS
            .get(ch)
            .or_else(|| font8x8::LATIN_FONTS.get(ch));
        let Some(g) = g else { continue };
        for (row, bits) in g.iter().enumerate() {
            for col in 0..8 {
                if bits >> col & 1 == 1 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            put(
                                img,
                                x + (i as i32 * 8 + col) * scale + sx,
                                y + row as i32 * scale + sy,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn draw_qr(img: &mut GrayImage, data: &str, x: i32, y: i32, m: i32) {
    let Ok(code) = QrCode::new(data.as_bytes()) else {
        return;
    };
    let n = code.width() as i32;
    for (i, c) in code.to_colors().into_iter().enumerate() {
        if c == Color::Dark {
            let (cx, cy) = (i as i32 % n, i as i32 / n);
            for dy in 0..m {
                for dx in 0..m {
                    put(img, x + cx * m + dx, y + cy * m + dy);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_size_and_ink() {
        let img =
            render("^XA^PW384^LL100^FO10,10^A0N,32,32^FDHi^FS^FO200,10^BQN,2,3^FDQA,hello^FS^XZ");
        assert_eq!(img.dimensions(), (384, 100));
        assert!(img.pixels().any(|p| p[0] == 0));
    }
}
