use niimbot_zpl::designer::{self, Element, Kind};
use niimbot_zpl::{data, protocol::Printer, transport, zpl};
use slint::{
    ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel,
    Weak,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

slint::include_modules!();

const SAMPLE_ZPL: &str =
    "^XA\n^PW384\n^LL240\n^FO20,20^A0N,40,40^FD{{nome}}^FS\n^FO20,90^BQN,2,5^FDQA,{{nome}}^FS\n^XZ";

enum Cmd {
    Batch(Vec<image::GrayImage>, u8, u16),
    Calibrate,
}

fn to_image(g: &image::GrayImage) -> Image {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(g.width(), g.height());
    for (d, p) in buf.make_mut_slice().iter_mut().zip(g.pixels()) {
        *d = Rgba8Pixel {
            r: p[0],
            g: p[0],
            b: p[0],
            a: 255,
        };
    }
    Image::from_rgba8(buf)
}

fn ui(app: &Weak<App>, f: impl FnOnce(App) + Send + 'static) {
    let _ = app.upgrade_in_event_loop(f);
}

/// Owns the connection: auto-detects the printer, polls status, runs print jobs.
fn worker(app: Weak<App>, rx: mpsc::Receiver<Cmd>) {
    let mut conn: Option<Printer<transport::UsbSerial>> = None;
    loop {
        let job = rx.recv_timeout(Duration::from_secs(2));
        if let Err(mpsc::RecvTimeoutError::Disconnected) = job {
            return;
        }
        if conn.is_none() {
            if let Some(path) = transport::find_niimbot() {
                conn = transport::UsbSerial::open(&path).ok().map(Printer::new);
            }
        }
        let Some(p) = conn.as_mut() else {
            ui(&app, |a| {
                a.set_connected(false);
                a.set_busy(false);
                a.set_conn_text("Impressora não encontrada".into());
            });
            continue;
        };
        let mut message = None;
        match job {
            Ok(Cmd::Batch(imgs, density, copies)) => {
                let n = imgs.len();
                message = Some(format!(
                    "Impresso ({n} etiqueta{})",
                    if n == 1 { "" } else { "s" }
                ));
                for (i, img) in imgs.iter().enumerate() {
                    let msg = format!("Imprimindo {}/{n}…", i + 1);
                    ui(&app, move |a| a.set_message(msg.into()));
                    if let Err(e) = p.print(img, density, copies) {
                        message = Some(format!("Erro na etiqueta {}: {e}", i + 1));
                        break;
                    }
                }
            }
            Ok(Cmd::Calibrate) => {
                message = Some(match p.calibrate() {
                    Ok(()) => "Calibrada".to_string(),
                    Err(e) => format!("Erro ao calibrar: {e}"),
                });
            }
            Err(_) => {}
        }
        match (p.battery(), p.firmware()) {
            (Ok(b), fw) => {
                let fw = fw.unwrap_or_default();
                ui(&app, move |a| {
                    a.set_connected(true);
                    a.set_busy(false);
                    a.set_conn_text(format!("B1 conectada · fw {fw}").into());
                    a.set_battery_text(format!("{b}%").into());
                    if let Some(m) = message {
                        a.set_message(m.into());
                    }
                });
            }
            _ => {
                conn = None; // unplugged or unresponsive: re-detect next round
                ui(&app, |a| {
                    a.set_connected(false);
                    a.set_busy(false);
                    a.set_conn_text("Conexão perdida".into());
                });
            }
        }
    }
}

struct State {
    els: Vec<Element>,
    sel: Option<usize>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone)]
struct Ctx {
    app: Weak<App>,
    st: Rc<RefCell<State>>,
    model: Rc<VecModel<El>>,
}

impl Ctx {
    fn dots(a: &App) -> (i32, i32) {
        ((a.get_label_w_mm() * 8).min(384), a.get_label_h_mm() * 8)
    }

    /// ZPL for one data row (None = no data), with variables substituted.
    fn zpl_for(&self, a: &App, row: Option<usize>) -> String {
        let st = self.st.borrow();
        let tpl = if a.get_mode() == 0 {
            let (w, h) = Self::dots(a);
            designer::to_zpl(&st.els, w, h)
        } else {
            a.get_zpl().to_string()
        };
        data::substitute(&tpl, &st.headers, row.and_then(|r| st.rows.get(r)))
    }

    fn preview_row(&self, a: &App) -> Option<usize> {
        let n = self.st.borrow().rows.len();
        (n > 0).then(|| (a.get_preview_row().max(1) as usize - 1).min(n - 1))
    }

    /// Re-renders preview, overlay boxes and the property panel from the state.
    fn refresh(&self) {
        let a = self.app.upgrade().unwrap();
        let row = self.preview_row(&a);
        let img = zpl::render(&self.zpl_for(&a, row));
        a.set_canvas_w(img.width() as i32);
        a.set_canvas_h(img.height() as i32);
        a.set_preview(to_image(&img));

        let st = self.st.borrow();
        let boxes: Vec<El> = if a.get_mode() == 0 {
            st.els
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let t =
                        data::substitute(&e.text, &st.headers, row.and_then(|r| st.rows.get(r)));
                    let (x, y, w, h) = e.bbox(&t);
                    El {
                        x: x as f32,
                        y: y as f32,
                        w: w as f32,
                        h: h as f32,
                        selected: st.sel == Some(i),
                    }
                })
                .collect()
        } else {
            vec![]
        };
        // Update in place: replacing the model would cancel an ongoing drag.
        if self.model.row_count() != boxes.len() {
            self.model.set_vec(boxes);
        } else {
            for (i, b) in boxes.into_iter().enumerate() {
                self.model.set_row_data(i, b);
            }
        }

        match st.sel.and_then(|i| st.els.get(i)) {
            Some(e) => {
                a.set_sel(st.sel.unwrap() as i32);
                a.set_sel_kind(match e.kind {
                    Kind::Text => 0,
                    Kind::Qr => 1,
                    Kind::Box => 2,
                });
                if a.get_sel_text() != e.text.as_str() {
                    a.set_sel_text(e.text.clone().into());
                }
                a.set_sel_size(e.size);
                a.set_sel_w(e.w);
                a.set_sel_h(e.h);
            }
            None => a.set_sel(-1),
        }

        let hs: Vec<SharedString> = st.headers.iter().map(|h| h.as_str().into()).collect();
        a.set_headers(ModelRc::new(VecModel::from(hs)));
        a.set_row_count(st.rows.len() as i32);
        a.set_data_info(if st.rows.is_empty() {
            "Nenhum dado importado. Use {{coluna}} nos textos e importe um CSV (1ª linha = nomes das colunas).".into()
        } else {
            format!("{} linhas · colunas: {}", st.rows.len(), st.headers.join(", ")).into()
        });
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    app.set_zpl(SAMPLE_ZPL.into());
    let model = Rc::new(VecModel::<El>::default());
    app.set_els(ModelRc::from(model.clone()));
    let st = Rc::new(RefCell::new(State {
        els: vec![Element::new(Kind::Text)],
        sel: Some(0),
        headers: vec![],
        rows: vec![],
    }));
    let ctx = Ctx {
        app: app.as_weak(),
        st,
        model,
    };
    ctx.refresh();

    let c = ctx.clone();
    app.on_render(move || c.refresh());
    let c = ctx.clone();
    app.on_row_changed(move || c.refresh());

    let c = ctx.clone();
    app.on_mode_changed(move || {
        // Switching to ZPL carries the current design over as editable code.
        let a = c.app.upgrade().unwrap();
        if a.get_mode() == 1 {
            let (w, h) = Ctx::dots(&a);
            a.set_zpl(designer::to_zpl(&c.st.borrow().els, w, h).into());
        }
        c.refresh();
    });

    let c = ctx.clone();
    app.on_add(move |k| {
        let kind = [Kind::Text, Kind::Qr, Kind::Box][k as usize];
        {
            let mut st = c.st.borrow_mut();
            st.els.push(Element::new(kind));
            st.sel = Some(st.els.len() - 1);
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_select(move |i| {
        c.st.borrow_mut().sel = (i >= 0).then_some(i as usize);
        c.refresh();
    });
    let c = ctx.clone();
    app.on_remove(move || {
        {
            let mut st = c.st.borrow_mut();
            if let Some(i) = st.sel.take() {
                st.els.remove(i);
            }
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_move_element(move |i, dx, dy| {
        {
            let mut st = c.st.borrow_mut();
            let (w, h) = Ctx::dots(&c.app.upgrade().unwrap());
            if let Some(e) = st.els.get_mut(i as usize) {
                e.x = (e.x + dx.round() as i32).clamp(0, w - 1);
                e.y = (e.y + dy.round() as i32).clamp(0, h - 1);
            }
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_set_text(move |t| {
        {
            let mut st = c.st.borrow_mut();
            if let Some(e) = st.sel.and_then(|i| st.els.get_mut(i)) {
                e.text = t.to_string();
            }
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_set_prop(move |which, v| {
        {
            let mut st = c.st.borrow_mut();
            if let Some(e) = st.sel.and_then(|i| st.els.get_mut(i)) {
                match which {
                    0 => e.size = v,
                    1 => e.w = v,
                    _ => e.h = v,
                }
            }
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_insert_var(move |name| {
        let a = c.app.upgrade().unwrap();
        let tag = format!("{{{{{name}}}}}");
        if a.get_mode() == 1 {
            a.set_zpl(format!("{}{tag}", a.get_zpl()).into());
        } else {
            let mut st = c.st.borrow_mut();
            if let Some(e) = st.sel.and_then(|i| st.els.get_mut(i)) {
                if e.kind != Kind::Box {
                    e.text.push_str(&tag);
                }
            }
        }
        c.refresh();
    });

    let c = ctx.clone();
    app.on_import_data(move || {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Dados (CSV/TXT)", &["csv", "txt", "tsv"])
            .pick_file()
        else {
            return;
        };
        let a = c.app.upgrade().unwrap();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let (headers, rows) = data::parse_csv(&text);
                let mut st = c.st.borrow_mut();
                st.headers = headers;
                st.rows = rows;
                a.set_preview_row(1);
            }
            Err(e) => a.set_message(format!("Erro ao ler arquivo: {e}").into()),
        }
        c.refresh();
    });
    let c = ctx.clone();
    app.on_clear_data(move || {
        {
            let mut st = c.st.borrow_mut();
            st.headers.clear();
            st.rows.clear();
        }
        c.refresh();
    });

    let (tx, rx) = mpsc::channel();
    let weak = app.as_weak();
    std::thread::spawn(move || worker(weak, rx));

    let txc = tx.clone();
    let w = app.as_weak();
    app.on_calibrate(move || {
        let a = w.upgrade().unwrap();
        a.set_busy(true);
        a.set_message("Calibrando… vão gastar ~4 etiquetas".into());
        let _ = txc.send(Cmd::Calibrate);
    });

    let c = ctx.clone();
    app.on_print(move || {
        let a = c.app.upgrade().unwrap();
        let n = c.st.borrow().rows.len();
        let imgs: Vec<_> = if n == 0 {
            vec![zpl::render(&c.zpl_for(&a, None))]
        } else {
            (0..n)
                .map(|r| zpl::render(&c.zpl_for(&a, Some(r))))
                .collect()
        };
        a.set_busy(true);
        a.set_message("".into());
        let _ = tx.send(Cmd::Batch(
            imgs,
            a.get_density() as u8,
            a.get_copies() as u16,
        ));
    });
    app.run()
}
