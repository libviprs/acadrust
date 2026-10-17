//! The R12 (AC1009) layout of a DXF file.
//!
//! R12 redates the handle-based object model. Its table records carry no
//! handles, no `330` owner pointers and no `100` subclass markers, it has no
//! CLASSES or OBJECTS section, and it knows a short list of entities (no
//! LWPOLYLINE, MTEXT, SPLINE, HATCH ...). The section writer produces the R13
//! structure, so for AC1009 its output is recorded as code/value pairs, taken
//! down to the R12 layout here, and played into the real (ASCII or binary)
//! writer. That keeps the R12 rules in one file instead of a version test in
//! every entity writer.
//!
//! Entities R12 cannot carry are dropped (a paper space VIEWPORT too: R12 keeps
//! it in MVIEW extended data, which is not the entity the writer has), except two that have an R12 form:
//! an LWPOLYLINE becomes a POLYLINE with VERTEX records and a SEQEND, and an
//! MTEXT becomes one TEXT per paragraph (what AutoCAD does when it saves as
//! R12, minus the formatting). Nothing else is invented.
//! See libviprs/acadsharp-rs-tests#111.

use super::stream_writer::DxfStreamWriter;
use crate::error::Result;
use crate::types::Handle;

/// The value of one recorded pair, with the writer call that produced it.
#[derive(Clone, Debug)]
pub(crate) enum Val {
    Str(String),
    Xrecord(String),
    Byte(u8),
    I16(i16),
    I32(i32),
    I64(i64),
    Double(f64),
    Bool(bool),
    Handle(Handle),
    Binary(Vec<u8>),
}

/// One recorded code/value pair.
#[derive(Clone, Debug)]
pub(crate) struct Pair {
    pub code: i32,
    pub val: Val,
}

impl Pair {
    fn s(code: i32, v: &str) -> Pair {
        Pair {
            code,
            val: Val::Str(v.to_string()),
        }
    }
    fn d(code: i32, v: f64) -> Pair {
        Pair {
            code,
            val: Val::Double(v),
        }
    }
    fn i(code: i32, v: i16) -> Pair {
        Pair {
            code,
            val: Val::I16(v),
        }
    }
    fn text(&self) -> Option<&str> {
        match &self.val {
            Val::Str(s) | Val::Xrecord(s) => Some(s),
            _ => None,
        }
    }
    fn num(&self) -> Option<f64> {
        match &self.val {
            Val::Double(v) => Some(*v),
            Val::I16(v) => Some(f64::from(*v)),
            Val::I32(v) => Some(f64::from(*v)),
            Val::I64(v) => Some(*v as f64),
            Val::Byte(v) => Some(f64::from(*v)),
            Val::Bool(v) => Some(f64::from(u8::from(*v))),
            _ => None,
        }
    }
    fn is(&self, code: i32, text: &str) -> bool {
        self.code == code && self.text() == Some(text)
    }
}

/// A [`DxfStreamWriter`] that keeps what it is given.
pub(crate) struct PairRecorder {
    pairs: Vec<Pair>,
    binary: bool,
}

impl PairRecorder {
    pub fn new(binary: bool) -> Self {
        Self {
            pairs: Vec::new(),
            binary,
        }
    }

    pub fn into_pairs(self) -> Vec<Pair> {
        self.pairs
    }

    fn push(&mut self, code: i32, val: Val) -> Result<()> {
        self.pairs.push(Pair { code, val });
        Ok(())
    }
}

impl DxfStreamWriter for PairRecorder {
    fn is_binary(&self) -> bool {
        self.binary
    }
    fn write_string(&mut self, code: i32, value: &str) -> Result<()> {
        self.push(code, Val::Str(value.to_string()))
    }
    fn write_xrecord_string(&mut self, code: i32, value: &str) -> Result<()> {
        self.push(code, Val::Xrecord(value.to_string()))
    }
    fn write_byte(&mut self, code: i32, value: u8) -> Result<()> {
        self.push(code, Val::Byte(value))
    }
    fn write_i16(&mut self, code: i32, value: i16) -> Result<()> {
        self.push(code, Val::I16(value))
    }
    fn write_i32(&mut self, code: i32, value: i32) -> Result<()> {
        self.push(code, Val::I32(value))
    }
    fn write_i64(&mut self, code: i32, value: i64) -> Result<()> {
        self.push(code, Val::I64(value))
    }
    fn write_double(&mut self, code: i32, value: f64) -> Result<()> {
        self.push(code, Val::Double(value))
    }
    fn write_bool(&mut self, code: i32, value: bool) -> Result<()> {
        self.push(code, Val::Bool(value))
    }
    fn write_handle(&mut self, code: i32, handle: Handle) -> Result<()> {
        self.push(code, Val::Handle(handle))
    }
    fn write_binary(&mut self, code: i32, data: &[u8]) -> Result<()> {
        self.push(code, Val::Binary(data.to_vec()))
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Play pairs into a writer.
pub(crate) fn replay<W: DxfStreamWriter>(pairs: &[Pair], w: &mut W) -> Result<()> {
    for p in pairs {
        match &p.val {
            Val::Str(s) => w.write_string(p.code, s)?,
            Val::Xrecord(s) => w.write_xrecord_string(p.code, s)?,
            Val::Byte(v) => w.write_byte(p.code, *v)?,
            Val::I16(v) => w.write_i16(p.code, *v)?,
            Val::I32(v) => w.write_i32(p.code, *v)?,
            Val::I64(v) => w.write_i64(p.code, *v)?,
            Val::Double(v) => w.write_double(p.code, *v)?,
            Val::Bool(v) => w.write_bool(p.code, *v)?,
            Val::Handle(v) => w.write_handle(p.code, *v)?,
            Val::Binary(v) => w.write_binary(p.code, v)?,
        }
    }
    Ok(())
}

/// Header variables R12 defines. Anything else is left out: a reader that
/// applies R12 rules has no use for `$CELWEIGHT`, and AutoCAD flags a variable
/// it does not know.
const R12_HEADER: &[&str] = &[
    "$ACADVER", "$DWGCODEPAGE", "$INSBASE", "$EXTMIN", "$EXTMAX", "$LIMMIN", "$LIMMAX",
    "$ORTHOMODE", "$REGENMODE", "$FILLMODE", "$QTEXTMODE", "$MIRRTEXT", "$DRAGMODE", "$LTSCALE",
    "$OSMODE", "$ATTMODE", "$TEXTSIZE", "$TRACEWID", "$TEXTSTYLE", "$CLAYER", "$CELTYPE",
    "$CECOLOR", "$CELTSCALE", "$DISPSILH", "$DIMPOST", "$DIMAPOST", "$DIMSCALE", "$DIMASZ",
    "$DIMEXO", "$DIMDLI", "$DIMRND", "$DIMDLE", "$DIMEXE", "$DIMTP", "$DIMTM", "$DIMTXT",
    "$DIMCEN", "$DIMTSZ", "$DIMTOL", "$DIMLIM", "$DIMTIH", "$DIMTOH", "$DIMSE1", "$DIMSE2",
    "$DIMTAD", "$DIMZIN", "$DIMBLK", "$DIMASO", "$DIMSHO", "$DIMALT", "$DIMALTD", "$DIMALTF",
    "$DIMLFAC", "$DIMTOFL", "$DIMTVP", "$DIMTIX", "$DIMSOXD", "$DIMSAH", "$DIMBLK1", "$DIMBLK2",
    "$DIMCLRD", "$DIMCLRE", "$DIMCLRT", "$DIMTFAC", "$DIMGAP", "$LUNITS", "$LUPREC",
    "$SKETCHINC", "$FILLETRAD", "$AUNITS", "$AUPREC", "$MENU", "$ELEVATION", "$PELEVATION",
    "$THICKNESS", "$LIMCHECK", "$BLIPMODE", "$CHAMFERA", "$CHAMFERB", "$SKPOLY", "$TDCREATE",
    "$TDUPDATE", "$TDINDWG", "$TDUSRTIMER", "$USRTIMER", "$ANGBASE", "$ANGDIR", "$PDMODE",
    "$PDSIZE", "$PLINEWID", "$COORDS", "$SPLFRAME", "$SPLINETYPE", "$SPLINESEGS", "$ATTDIA",
    "$ATTREQ", "$SURFTAB1", "$SURFTAB2", "$SURFTYPE", "$SURFU", "$SURFV", "$UCSNAME",
    "$UCSORG", "$UCSXDIR", "$UCSYDIR", "$PUCSNAME", "$PUCSORG", "$PUCSXDIR", "$PUCSYDIR",
    "$USERI1", "$USERI2", "$USERI3", "$USERI4", "$USERI5", "$USERR1", "$USERR2", "$USERR3",
    "$USERR4", "$USERR5", "$WORLDVIEW", "$SHADEDGE", "$SHADEDIF", "$TILEMODE", "$MAXACTVP",
    "$PLIMCHECK", "$PEXTMIN", "$PEXTMAX", "$PLIMMIN", "$PLIMMAX", "$UNITMODE", "$VISRETAIN",
    "$PLINEGEN", "$PSLTSCALE", "$TREEDEPTH",
];

/// Tables R12 has. BLOCK_RECORD and DIMSTYLE came with R13.
const R12_TABLES: &[&str] = &["VPORT", "LTYPE", "LAYER", "STYLE", "VIEW", "UCS", "APPID"];

/// Entities R12 has.
const R12_ENTITIES: &[&str] = &[
    "LINE", "POINT", "CIRCLE", "ARC", "TRACE", "SOLID", "TEXT", "SHAPE", "INSERT", "ATTDEF",
    "ATTRIB", "POLYLINE", "VERTEX", "SEQEND", "3DFACE", "DIMENSION", "BLOCK",
    "ENDBLK",
];

/// Whether a group code is one R12 does not have, on any record.
fn r13_only_code(code: i32) -> bool {
    matches!(
        code,
        5 | 100 | 102 | 105 | 330..=369 | 370..=389 | 390..=399 | 420..=449 | 284 | 480 | 481
        | 1005
    ) || (290..=299).contains(&code)
}

/// Take the code/value pairs of an R13+ DXF down to the R12 layout.
pub(crate) fn downgrade(pairs: Vec<Pair>) -> Vec<Pair> {
    let mut out = Vec::with_capacity(pairs.len() / 2);
    let mut i = 0;
    while i < pairs.len() {
        if pairs[i].is(0, "EOF") {
            out.push(pairs[i].clone());
            break;
        }
        if !pairs[i].is(0, "SECTION") || i + 1 >= pairs.len() {
            i += 1;
            continue;
        }
        let name = pairs[i + 1].text().unwrap_or("").to_string();
        let mut end = i + 2;
        while end < pairs.len() && !pairs[end].is(0, "ENDSEC") {
            end += 1;
        }
        let body = &pairs[(i + 2).min(end)..end];
        let kept = match name.as_str() {
            "HEADER" => Some(header(body)),
            "TABLES" => Some(tables(body)),
            "BLOCKS" => Some(blocks(body)),
            "ENTITIES" => Some(entities(body)),
            // CLASSES, OBJECTS and ACDSDATA do not exist in R12.
            _ => None,
        };
        if let Some(kept) = kept {
            out.push(Pair::s(0, "SECTION"));
            out.push(Pair::s(2, &name));
            out.extend(kept);
            out.push(Pair::s(0, "ENDSEC"));
        }
        i = end + 1;
    }
    out
}

fn header(body: &[Pair]) -> Vec<Pair> {
    let mut out = Vec::new();
    let mut keep = false;
    for p in body {
        if p.code == 9 {
            let n = p.text().unwrap_or("");
            keep = R12_HEADER.contains(&n);
            if keep {
                out.push(p.clone());
            }
        } else if keep {
            out.push(p.clone());
        }
    }
    out
}

/// Split a run of pairs into records, each starting at a group 0.
fn records(body: &[Pair]) -> Vec<&[Pair]> {
    let mut out = Vec::new();
    let mut start = None;
    for (n, p) in body.iter().enumerate() {
        if p.code == 0 {
            if let Some(s) = start {
                out.push(&body[s..n]);
            }
            start = Some(n);
        }
    }
    if let Some(s) = start {
        out.push(&body[s..]);
    }
    out
}

/// A record without the groups R12 lacks, and with the `102 {...}` groups
/// (reactors, extension dictionary) removed whole.
fn strip(record: &[Pair], also: &[i32]) -> Vec<Pair> {
    record
        .iter()
        .filter(|p| !r13_only_code(p.code) && !also.contains(&p.code))
        .cloned()
        .collect()
}

fn tables(body: &[Pair]) -> Vec<Pair> {
    let mut out = Vec::new();
    let mut keep = false;
    for rec in records(body) {
        let head = rec[0].text().unwrap_or("");
        match head {
            "TABLE" => {
                let t = rec
                    .iter()
                    .find(|p| p.code == 2)
                    .and_then(Pair::text)
                    .unwrap_or("");
                keep = R12_TABLES.contains(&t);
                if keep {
                    // A table header is `0 TABLE`, `2 name`, `70 count`.
                    out.push(rec[0].clone());
                    out.push(Pair::s(2, t));
                    if let Some(c) = rec.iter().find(|p| p.code == 70) {
                        out.push(c.clone());
                    }
                }
            }
            "ENDTAB" => {
                if keep {
                    out.push(rec[0].clone());
                }
                keep = false;
            }
            _ if keep => {
                // Complex linetype elements (text, shapes) are R13.
                out.extend(strip(rec, &[74, 75, 46, 50, 44, 45, 9, 1071]));
            }
            _ => {}
        }
    }
    out
}

fn blocks(body: &[Pair]) -> Vec<Pair> {
    let mut out = Vec::new();
    // Paper space layouts other than the one are an R2000 idea. A block of one
    // is written whole or not at all.
    let mut skip = false;
    for rec in records(body) {
        let head = rec[0].text().unwrap_or("");
        if head == "BLOCK" {
            let name = rec
                .iter()
                .find(|p| p.code == 2)
                .and_then(Pair::text)
                .unwrap_or("");
            let up = name.to_ascii_uppercase();
            skip = up.starts_with("*PAPER_SPACE")
                && up.len() > "*PAPER_SPACE".len()
                && up["*PAPER_SPACE".len()..].bytes().all(|b| b.is_ascii_digit());
        }
        if !skip {
            out.extend(entity(rec));
        }
        if head == "ENDBLK" {
            skip = false;
        }
    }
    out
}

fn entities(body: &[Pair]) -> Vec<Pair> {
    let mut out = Vec::new();
    for rec in records(body) {
        out.extend(entity(rec));
    }
    out
}

/// One entity record in R12 form (nothing, one or several records).
fn entity(rec: &[Pair]) -> Vec<Pair> {
    let name = rec[0].text().unwrap_or("");
    match name {
        "LWPOLYLINE" => lwpolyline(rec),
        "MTEXT" => mtext(rec),
        "DIMENSION" => strip(rec, &[3, 71, 72, 41, 42, 53]),
        "ATTRIB" | "ATTDEF" => strip(rec, &[280]),
        n if R12_ENTITIES.contains(&n) => strip(rec, &[]),
        _ => Vec::new(),
    }
}

/// The groups every entity keeps whatever it is: layer, linetype, colour,
/// space and the like. The geometry is replaced by the caller.
fn common(rec: &[Pair]) -> Vec<Pair> {
    rec.iter()
        .filter(|p| matches!(p.code, 8 | 6 | 62 | 48 | 60 | 67 | 39))
        .cloned()
        .collect()
}

fn lwpolyline(rec: &[Pair]) -> Vec<Pair> {
    let mut flags = 0i16;
    let mut width = 0.0;
    let mut elevation = 0.0;
    let mut normal = [0.0, 0.0, 1.0];
    struct V {
        x: f64,
        y: f64,
        sw: Option<f64>,
        ew: Option<f64>,
        bulge: Option<f64>,
    }
    let mut vs: Vec<V> = Vec::new();
    for p in rec {
        let Some(n) = p.num() else { continue };
        match p.code {
            70 => flags = n as i16,
            43 => width = n,
            38 => elevation = n,
            210 => normal[0] = n,
            220 => normal[1] = n,
            230 => normal[2] = n,
            10 => vs.push(V {
                x: n,
                y: 0.0,
                sw: None,
                ew: None,
                bulge: None,
            }),
            20 => {
                if let Some(v) = vs.last_mut() {
                    v.y = n;
                }
            }
            40 => {
                if let Some(v) = vs.last_mut() {
                    v.sw = Some(n);
                }
            }
            41 => {
                if let Some(v) = vs.last_mut() {
                    v.ew = Some(n);
                }
            }
            42 => {
                if let Some(v) = vs.last_mut() {
                    v.bulge = Some(n);
                }
            }
            _ => {}
        }
    }
    let layer = rec
        .iter()
        .find(|p| p.code == 8)
        .and_then(Pair::text)
        .unwrap_or("0")
        .to_string();
    let mut out = vec![Pair::s(0, "POLYLINE")];
    out.extend(common(rec));
    out.push(Pair::i(66, 1));
    out.push(Pair::d(10, 0.0));
    out.push(Pair::d(20, 0.0));
    out.push(Pair::d(30, elevation));
    // 1 closed, 128 linetype generation: the two bits the light form shares.
    out.push(Pair::i(70, flags & (1 | 128)));
    if width != 0.0 {
        out.push(Pair::d(40, width));
        out.push(Pair::d(41, width));
    }
    if normal != [0.0, 0.0, 1.0] {
        out.push(Pair::d(210, normal[0]));
        out.push(Pair::d(220, normal[1]));
        out.push(Pair::d(230, normal[2]));
    }
    for v in vs {
        out.push(Pair::s(0, "VERTEX"));
        out.push(Pair::s(8, &layer));
        out.push(Pair::d(10, v.x));
        out.push(Pair::d(20, v.y));
        out.push(Pair::d(30, elevation));
        if let Some(w) = v.sw {
            out.push(Pair::d(40, w));
        }
        if let Some(w) = v.ew {
            out.push(Pair::d(41, w));
        }
        if let Some(b) = v.bulge.filter(|b| *b != 0.0) {
            out.push(Pair::d(42, b));
        }
    }
    out.push(Pair::s(0, "SEQEND"));
    out.push(Pair::s(8, &layer));
    out
}

/// Plain text of an MTEXT value: paragraphs split at `\P`, the inline codes
/// that only format dropped.
fn mtext_paragraphs(value: &str) -> Vec<String> {
    let mut paragraphs = vec![String::new()];
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('P') | Some('p') => paragraphs.push(String::new()),
                Some('~') => paragraphs.last_mut().unwrap().push(' '),
                Some('\\') => paragraphs.last_mut().unwrap().push('\\'),
                Some('{') => paragraphs.last_mut().unwrap().push('{'),
                Some('}') => paragraphs.last_mut().unwrap().push('}'),
                // \f..; \H..; \C..; \A..; \T..; \Q..; \W.. \L \l \O \o \K \k
                Some(k) if "fFHhCcAaTtQqWwSs".contains(k) => {
                    for d in chars.by_ref() {
                        if d == ';' {
                            break;
                        }
                    }
                }
                Some(_) | None => {}
            },
            '{' | '}' => {}
            _ => paragraphs.last_mut().unwrap().push(c),
        }
    }
    paragraphs
}

fn mtext(rec: &[Pair]) -> Vec<Pair> {
    let mut value = String::new();
    let mut chunks = String::new();
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    let (mut height, mut rotation) = (1.0, 0.0);
    let mut xdir: Option<(f64, f64)> = None;
    let mut style = "STANDARD".to_string();
    let mut seen_columns = false;
    for p in rec {
        match p.code {
            3 => chunks.push_str(p.text().unwrap_or("")),
            1 => value = p.text().unwrap_or("").to_string(),
            7 => style = p.text().unwrap_or("STANDARD").to_string(),
            10 => x = p.num().unwrap_or(0.0),
            20 => y = p.num().unwrap_or(0.0),
            30 => z = p.num().unwrap_or(0.0),
            40 => height = p.num().unwrap_or(1.0),
            75 => seen_columns = true,
            // After the column marker group 50 is a column height.
            50 if !seen_columns => rotation = p.num().unwrap_or(0.0),
            11 => xdir = Some((p.num().unwrap_or(1.0), xdir.map_or(0.0, |d| d.1))),
            21 => xdir = Some((xdir.map_or(1.0, |d| d.0), p.num().unwrap_or(0.0))),
            _ => {}
        }
    }
    chunks.push_str(&value);
    let angle = match xdir {
        Some((dx, dy)) if dx != 0.0 || dy != 0.0 => dy.atan2(dx),
        _ => rotation.to_radians(),
    };
    let common = common(rec);
    let mut out = Vec::new();
    for (n, line) in mtext_paragraphs(&chunks).into_iter().enumerate() {
        // AutoCAD's default line pitch is 5/3 of the text height.
        let drop = n as f64 * height * 5.0 / 3.0;
        out.push(Pair::s(0, "TEXT"));
        out.extend(common.iter().cloned());
        out.push(Pair::d(10, x + drop * angle.sin()));
        out.push(Pair::d(20, y - drop * angle.cos()));
        out.push(Pair::d(30, z));
        out.push(Pair::d(40, height));
        out.push(Pair::s(1, &line));
        if angle != 0.0 {
            out.push(Pair::d(50, angle.to_degrees()));
        }
        out.push(Pair::s(7, &style));
    }
    out
}
