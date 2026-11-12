//! Drag-and-drop component model. Everything is converted to ZPL under the hood.
use crate::zpl;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Text,
    Qr,
    Box,
}

#[derive(Clone, Debug)]
pub struct Element {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
    /// Box only.
    pub w: i32,
    pub h: i32,
    /// Text: height in dots. Qr: module size. Box: border thickness.
    pub size: i32,
    pub text: String,
}

impl Element {
    pub fn new(kind: Kind) -> Self {
        match kind {
            Kind::Text => Self {
                kind,
                x: 16,
                y: 16,
                w: 0,
                h: 0,
                size: 40,
                text: "{{nome}}".into(),
            },
            Kind::Qr => Self {
                kind,
                x: 16,
                y: 16,
                w: 0,
                h: 0,
                size: 4,
                text: "https://example.com".into(),
            },
            Kind::Box => Self {
                kind,
                x: 8,
                y: 8,
                w: 120,
                h: 80,
                size: 3,
                text: String::new(),
            },
        }
    }

    pub fn zpl(&self) -> String {
        let t = self.text.replace(['^', '~'], "");
        match self.kind {
            Kind::Text => format!(
                "^FO{},{}^A0N,{},{}^FD{}^FS\n",
                self.x, self.y, self.size, self.size, t
            ),
            Kind::Qr => format!(
                "^FO{},{}^BQN,2,{}^FDQA,{}^FS\n",
                self.x, self.y, self.size, t
            ),
            Kind::Box => format!(
                "^FO{},{}^GB{},{},{}^FS\n",
                self.x, self.y, self.w, self.h, self.size
            ),
        }
    }

    /// Bounding box (x, y, w, h) in dots; `text` is the already-substituted text.
    pub fn bbox(&self, text: &str) -> (i32, i32, i32, i32) {
        match self.kind {
            Kind::Text => {
                let (w, h) = zpl::text_extent(text, self.size);
                (self.x, self.y, w, h)
            }
            Kind::Qr => {
                let s = zpl::qr_size(text, self.size);
                (self.x, self.y, s, s)
            }
            Kind::Box => (self.x, self.y, self.w, self.h),
        }
    }
}

pub fn to_zpl(els: &[Element], w: i32, h: i32) -> String {
    let mut s = format!("^XA\n^PW{w}\n^LL{h}\n");
    for e in els {
        s += &e.zpl();
    }
    s + "^XZ"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_zpl_that_renders() {
        let z = to_zpl(
            &[
                Element::new(Kind::Text),
                Element::new(Kind::Qr),
                Element::new(Kind::Box),
            ],
            384,
            240,
        );
        assert!(z.contains("^BQN,2,4^FDQA,"));
        assert_eq!(zpl::render(&z).dimensions(), (384, 240));
    }
}
