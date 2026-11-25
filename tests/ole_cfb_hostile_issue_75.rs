// Issue #75: the OLE compound file reader behind OLE2FRAME previews must be
// bounded by the blob length. Every blob here is small and built in this file.
// A counting allocator (per thread) measures the peak and the biggest single
// block; a run that goes past a hard limit is parked and reported as a failure
// instead of taking the machine down.

use acadrust::compound_file::StructuredStoragePayload;
use acadrust::entities::{
    extract_presentation, ole_user_type, try_extract_presentation, OleError, OlePresentation,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Counting;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static CUR: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
    static BLOCK: Cell<usize> = const { Cell::new(0) };
}
static TRIPPED: AtomicBool = AtomicBool::new(false);

fn grow(by: usize, block: usize) {
    let _ = ON.try_with(|on| {
        if !on.get() {
            return;
        }
        let cur = CUR.with(|c| c.get()).saturating_add(by);
        if cur > LIMIT.with(|l| l.get()) || block > LIMIT.with(|l| l.get()) {
            TRIPPED.store(true, Ordering::SeqCst);
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
        CUR.with(|c| c.set(cur));
        PEAK.with(|p| p.set(p.get().max(cur)));
        BLOCK.with(|b| b.set(b.get().max(block)));
    });
}

fn shrink(by: usize) {
    let _ = ON.try_with(|on| {
        if on.get() {
            CUR.with(|c| c.set(c.get().saturating_sub(by)));
        }
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        grow(l.size(), l.size());
        System.alloc(l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        grow(l.size(), l.size());
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        if new > l.size() {
            grow(new - l.size(), new);
        } else {
            shrink(l.size() - new);
        }
        System.realloc(p, l, new)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        shrink(l.size());
        System.dealloc(p, l)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

static SERIAL: Mutex<()> = Mutex::new(());
const HARD_LIMIT: usize = 64 << 20;

/// Run `f` on its own thread with the allocation limit and a wall clock bound.
fn bounded<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    TRIPPED.store(false, Ordering::SeqCst);
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        LIMIT.with(|l| l.set(HARD_LIMIT));
        CUR.with(|c| c.set(0));
        ON.with(|on| on.set(true));
        let out = f();
        ON.with(|on| on.set(false));
        let _ = tx.send(out);
    });
    let start = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(25)) {
            Ok(v) => return Ok(v),
            Err(RecvTimeoutError::Disconnected) => return Err("the reader panicked".into()),
            Err(RecvTimeoutError::Timeout) => {
                if TRIPPED.load(Ordering::SeqCst) {
                    return Err(format!("allocated past {} MiB", HARD_LIMIT >> 20));
                }
                if start.elapsed() > Duration::from_secs(30) {
                    return Err("ran for over 30 s".into());
                }
            }
        }
    }
}

/// Peak live bytes and biggest single block of `f`, measured from now.
fn lap<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let start = CUR.with(|c| c.get());
    PEAK.with(|p| p.set(start));
    BLOCK.with(|b| b.set(0));
    let out = f();
    (out, PEAK.with(|p| p.get()) - start, BLOCK.with(|b| b.get()))
}

fn few_mb(len: usize) -> usize {
    (4 << 20).max(len * 8)
}

// ── Blob builders ───────────────────────────────────────────────────────────

const END: u32 = 0xFFFF_FFFE;
const FREE: u32 = 0xFFFF_FFFF;
const FATS: u32 = 0xFFFF_FFFD;
const DIFS: u32 = 0xFFFF_FFFC;
const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

fn put32(v: &mut [u8], off: usize, x: u32) {
    v[off..off + 4].copy_from_slice(&x.to_le_bytes());
}

fn header() -> Vec<u8> {
    let mut h = vec![0u8; 512];
    h[..8].copy_from_slice(&MAGIC);
    h[24..26].copy_from_slice(&0x3Eu16.to_le_bytes());
    h[26..28].copy_from_slice(&3u16.to_le_bytes());
    h[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
    h[30..32].copy_from_slice(&9u16.to_le_bytes());
    h[32..34].copy_from_slice(&6u16.to_le_bytes());
    put32(&mut h, 44, 1); // FAT sectors
    put32(&mut h, 48, 1); // directory start
    put32(&mut h, 56, 4096); // mini cutoff
    put32(&mut h, 60, END); // mini-FAT start
    put32(&mut h, 68, END); // DIFAT start
    for i in 0..109 {
        put32(&mut h, 76 + i * 4, FREE);
    }
    put32(&mut h, 76, 0); // FAT sector 0
    h
}

fn sector_of(entries: &[u32]) -> Vec<u8> {
    let mut s = vec![0u8; 512];
    for i in 0..128 {
        put32(&mut s, i * 4, *entries.get(i).unwrap_or(&FREE));
    }
    s
}

fn dir_entry(name: &str, kind: u8, start: u32, size: u32) -> Vec<u8> {
    let mut e = vec![0u8; 128];
    let units: Vec<u16> = name.encode_utf16().collect();
    for (i, u) in units.iter().enumerate() {
        e[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
    }
    e[64..66].copy_from_slice(&(((units.len() + 1) * 2) as u16).to_le_bytes());
    e[66] = kind;
    put32(&mut e, 68, FREE);
    put32(&mut e, 72, FREE);
    put32(&mut e, 76, FREE);
    put32(&mut e, 116, start);
    put32(&mut e, 120, size);
    e
}

fn dir_sector(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut s = Vec::new();
    for e in entries {
        s.extend_from_slice(e);
    }
    s.resize(512, 0);
    s
}

fn assemble(h: Vec<u8>, sectors: &[Vec<u8>]) -> Vec<u8> {
    let mut b = h;
    for s in sectors {
        b.extend_from_slice(s);
    }
    b
}

/// A CF_DIB OlePres stream (MS-OLEDS) around `body`.
fn olepres(body: &[u8]) -> Vec<u8> {
    let mut s = Vec::new();
    s.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    s.extend_from_slice(&8u32.to_le_bytes());
    s.extend_from_slice(&4u32.to_le_bytes());
    s.extend_from_slice(&[0u8; 24]);
    s.extend_from_slice(&(body.len() as u32).to_le_bytes());
    s.extend_from_slice(body);
    s
}

/// A valid blob with up to three streams: small ones (under 4096 bytes) go in
/// the mini stream, bigger ones get their own FAT chain.
fn valid_blob(streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut ministream = Vec::new();
    let mut minifat: Vec<u32> = Vec::new();
    let mut starts = Vec::new();
    let mut big = Vec::new();
    for (i, (_, data)) in streams.iter().enumerate() {
        if data.len() < 4096 {
            starts.push(minifat.len() as u32);
            let n = data.len().div_ceil(64);
            for k in 0..n {
                minifat.push(if k + 1 == n {
                    END
                } else {
                    minifat.len() as u32 + 1
                });
            }
            ministream.extend_from_slice(data);
            ministream.resize(minifat.len() * 64, 0);
        } else {
            starts.push(0);
            big.push(i);
        }
    }
    let ms_sectors = ministream.len().div_ceil(512);
    let mut fat = vec![FATS, END];
    let mut sectors = vec![Vec::new(), Vec::new()];
    let mut h = header();
    if !minifat.is_empty() {
        put32(&mut h, 60, 2);
        put32(&mut h, 64, 1);
        fat.push(END);
        sectors.push(sector_of(&minifat));
    }
    let root_start = if ms_sectors > 0 {
        fat.len() as u32
    } else {
        END
    };
    for k in 0..ms_sectors {
        fat.push(if k + 1 == ms_sectors {
            END
        } else {
            fat.len() as u32 + 1
        });
        let mut s = ministream[k * 512..((k + 1) * 512).min(ministream.len())].to_vec();
        s.resize(512, 0);
        sectors.push(s);
    }
    for i in big {
        let data = &streams[i].1;
        starts[i] = fat.len() as u32;
        let n = data.len().div_ceil(512);
        for k in 0..n {
            fat.push(if k + 1 == n {
                END
            } else {
                fat.len() as u32 + 1
            });
            let mut s = data[k * 512..((k + 1) * 512).min(data.len())].to_vec();
            s.resize(512, 0);
            sectors.push(s);
        }
    }
    let mut dir = vec![dir_entry(
        "Root Entry",
        5,
        root_start,
        ministream.len() as u32,
    )];
    for (i, (name, data)) in streams.iter().enumerate() {
        dir.push(dir_entry(name, 2, starts[i], data.len() as u32));
    }
    sectors[0] = sector_of(&fat);
    sectors[1] = dir_sector(&dir);
    assemble(h, &sectors)
}

fn body(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i % 251) as u8).collect()
}

const PRES: &str = "\u{2}OlePres000";

// ── The valid path still works ──────────────────────────────────────────────

#[test]
fn issue_75_valid_mini_stream_blob_still_reads() {
    let b = body(64);
    let blob = valid_blob(&[(PRES, olepres(&b))]);
    assert_eq!(
        try_extract_presentation(&blob),
        Ok(Some(OlePresentation::Dib(b.clone())))
    );
    assert_eq!(extract_presentation(&blob), Some(OlePresentation::Dib(b)));
}

#[test]
fn issue_75_valid_regular_stream_blob_still_reads() {
    let b = body(5000);
    let blob = valid_blob(&[(PRES, olepres(&b))]);
    assert_eq!(
        try_extract_presentation(&blob),
        Ok(Some(OlePresentation::Dib(b)))
    );
}

// ── Hostile blobs ───────────────────────────────────────────────────────────

/// A DIFAT sector that points at itself, with the maximum DIFAT count.
fn self_pointing_difat() -> Vec<u8> {
    let mut h = header();
    put32(&mut h, 68, 2);
    put32(&mut h, 72, 4096);
    let mut difat = vec![0u32; 127];
    difat.push(2);
    assemble(
        h,
        &[
            sector_of(&[FATS, END, DIFS]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            sector_of(&difat),
        ],
    )
}

#[test]
fn issue_75_self_pointing_difat_is_refused_within_a_few_mb() {
    let blob = self_pointing_difat();
    assert_eq!(blob.len(), 2048);
    let n = blob.len();
    let (res, peak, block) = bounded(move || lap(|| try_extract_presentation(&blob)))
        .map(|(r, p, b)| (r, p, b))
        .expect("bounded run");
    assert_eq!(res, Err(OleError::DifatLoop));
    assert!(peak <= few_mb(n), "peak {peak} bytes");
    assert!(block <= few_mb(n), "block {block} bytes");
}

#[test]
fn issue_75_self_pointing_difat_in_the_legacy_entry_points_stays_small() {
    let blob = self_pointing_difat();
    let n = blob.len();
    let (_, peak, _) = bounded(move || lap(|| (extract_presentation(&blob), ole_user_type(&blob))))
        .expect("bounded run");
    assert!(peak <= few_mb(n), "peak {peak} bytes");
}

#[test]
fn issue_75_a_fat_sector_listed_twice_is_refused() {
    let mut h = header();
    put32(&mut h, 44, 2);
    put32(&mut h, 80, 0);
    let blob = assemble(
        h,
        &[
            sector_of(&[FATS, END]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
        ],
    );
    assert_eq!(
        try_extract_presentation(&blob),
        Err(OleError::FatSectorRepeated)
    );
}

#[test]
fn issue_75_a_looping_directory_chain_is_refused() {
    let mut h = header();
    put32(&mut h, 48, 1);
    let blob = assemble(
        h,
        &[
            sector_of(&[FATS, 2, 1]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            dir_sector(&[]),
        ],
    );
    assert_eq!(try_extract_presentation(&blob), Err(OleError::ChainLoop));
}

#[test]
fn issue_75_a_looping_mini_fat_chain_is_refused() {
    let mut h = header();
    put32(&mut h, 60, 2);
    put32(&mut h, 64, 3);
    let blob = assemble(
        h,
        &[
            sector_of(&[FATS, END, 2]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            sector_of(&[]),
        ],
    );
    assert_eq!(try_extract_presentation(&blob), Err(OleError::ChainLoop));
}

#[test]
fn issue_75_a_looping_mini_fat_with_a_huge_count_does_not_run_away() {
    let mut h = header();
    put32(&mut h, 60, 2);
    put32(&mut h, 64, u32::MAX);
    let blob = assemble(
        h,
        &[
            sector_of(&[FATS, END, 2]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            sector_of(&[]),
        ],
    );
    let n = blob.len();
    let (res, peak, _) =
        bounded(move || lap(|| try_extract_presentation(&blob))).expect("bounded run");
    assert_eq!(res, Err(OleError::CountExceedsBlob));
    assert!(peak <= few_mb(n), "peak {peak} bytes");
}

#[test]
fn issue_75_header_counts_past_the_blob_are_refused() {
    for off in [44usize, 64, 72] {
        let mut h = header();
        put32(&mut h, off, u32::MAX);
        let blob = assemble(
            h,
            &[
                sector_of(&[FATS, END]),
                dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            ],
        );
        assert_eq!(
            try_extract_presentation(&blob),
            Err(OleError::CountExceedsBlob),
            "header offset {off}"
        );
    }
}

#[test]
fn issue_75_a_declared_stream_size_past_the_blob_reserves_nothing() {
    let blob = assemble(
        header(),
        &[
            sector_of(&[FATS, END, END]),
            dir_sector(&[
                dir_entry("Root Entry", 5, END, 0),
                dir_entry(PRES, 2, 2, 0xFFFF_FFF0),
            ]),
            vec![0u8; 512],
        ],
    );
    let n = blob.len();
    let (res, peak, block) =
        bounded(move || lap(|| try_extract_presentation(&blob))).expect("bounded run");
    assert_eq!(res, Err(OleError::SizeExceedsData));
    assert!(peak <= few_mb(n), "peak {peak} bytes");
    assert!(block <= few_mb(n), "block {block} bytes");
}

// ── Mutation loop over a valid blob ─────────────────────────────────────────

#[test]
fn issue_75_mutating_a_valid_blob_never_allocates_past_a_few_mb() {
    let valid = valid_blob(&[(PRES, olepres(&body(5000))), ("\u{1}CompObj", body(200))]);
    let n = valid.len();
    let (worst, runs) = bounded(move || {
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let specials = [
            0u32,
            1,
            2,
            3,
            0x7FFF_FFFF,
            0x8000_0000,
            0xFFFF_FFFC,
            0xFFFF_FFFD,
            0xFFFF_FFFE,
            0xFFFF_FFFF,
        ];
        let mut worst = 0usize;
        for _ in 0..20_000 {
            let mut b = valid.clone();
            for _ in 0..1 + rnd() % 4 {
                match rnd() % 5 {
                    0 => {
                        let i = rnd() as usize % b.len();
                        b[i] ^= 1 << (rnd() % 8);
                    }
                    1 => {
                        let o = (rnd() as usize % 128) * 4;
                        put32(&mut b, o, specials[rnd() as usize % specials.len()]);
                    }
                    2 => {
                        let o = (rnd() as usize % ((b.len() - 4) / 4)) * 4;
                        put32(&mut b, o, specials[rnd() as usize % specials.len()]);
                    }
                    3 => {
                        // point the DIFAT at a sector and ask for many
                        put32(&mut b, 68, (rnd() % 8) as u32);
                        put32(&mut b, 72, [16u32, 4096, u32::MAX][rnd() as usize % 3]);
                    }
                    _ => {
                        let keep = 512 + rnd() as usize % (b.len() - 511);
                        b.truncate(keep);
                    }
                }
            }
            let (_, peak, block) = lap(|| {
                let _ = try_extract_presentation(&b);
                let _ = extract_presentation(&b);
                let _ = ole_user_type(&b);
            });
            worst = worst.max(peak).max(block);
        }
        (worst, 20_000)
    })
    .expect("mutation loop");
    assert_eq!(runs, 20_000);
    assert!(worst <= few_mb(n), "worst allocation {worst} bytes");
}

// ── The structured-storage reader used for DXF and DWG payloads ─────────────

#[test]
fn issue_75_structured_storage_decode_bounds_a_self_pointing_difat() {
    // 1 MiB blob: a self-pointing DIFAT sector, an absurd FAT count.
    let mut h = header();
    put32(&mut h, 44, u32::MAX);
    put32(&mut h, 68, 2);
    put32(&mut h, 72, 2048);
    let mut difat = vec![0u32; 127];
    difat.push(2);
    let mut blob = assemble(
        h,
        &[
            sector_of(&[FATS, END, DIFS]),
            dir_sector(&[dir_entry("Root Entry", 5, END, 0)]),
            sector_of(&difat),
        ],
    );
    blob.resize(1 << 20, 0);
    let n = blob.len();
    let (payload, peak, block) =
        bounded(move || lap(|| StructuredStoragePayload::decode(&blob))).expect("bounded run");
    assert!(payload.compound_file.is_none());
    assert!(peak <= few_mb(n), "peak {peak} bytes");
    assert!(block <= few_mb(n), "block {block} bytes");
}

// ── Directory trees and shared chains (review of the first fix) ─────────────

/// A blob with a flat run of directory entries and the given data sectors.
/// `entries` is the whole directory (entry 0 is the root); `data_sectors` are
/// appended after the directory chain and listed in the FAT as one chain.
fn big_blob(entries: &[Vec<u8>], data_sectors: usize) -> Vec<u8> {
    let dir_sectors = entries.len().div_ceil(4);
    let mut k = 1;
    while k * 128 < k + dir_sectors + data_sectors {
        k += 1;
    }
    let mut fat = vec![FATS; k];
    for j in 0..dir_sectors {
        fat.push(if j + 1 == dir_sectors { END } else { (k + j) as u32 + 1 });
    }
    let chain_start = fat.len();
    for j in 0..data_sectors {
        fat.push(if j + 1 == data_sectors { END } else { (chain_start + j) as u32 + 1 });
    }
    let mut h = header();
    put32(&mut h, 44, k as u32);
    put32(&mut h, 48, k as u32);
    for i in 0..k {
        put32(&mut h, 76 + i * 4, i as u32);
    }
    let mut sectors: Vec<Vec<u8>> = fat.chunks(128).map(sector_of).collect();
    for c in entries.chunks(4) {
        sectors.push(dir_sector(c));
    }
    for _ in 0..data_sectors {
        sectors.push(vec![7u8; 512]);
    }
    assemble(h, &sectors)
}

/// 40000 entries below the root, each linked to the next through `link`
/// (left sibling or child), `kind` 2 for streams and 1 for storages.
fn chained_entries(link: usize, kind: u8) -> Vec<Vec<u8>> {
    let n = 40_000usize;
    let mut v = vec![dir_entry("Root Entry", 5, END, 0)];
    put32(&mut v[0], 76, 1);
    for i in 1..=n {
        let mut e = dir_entry("e", kind, END, 0);
        if i < n {
            put32(&mut e, link, i as u32 + 1);
        }
        v.push(e);
    }
    v
}

fn storage_depth(s: &acadrust::compound_file::CompoundStorage) -> usize {
    1 + s
        .entries
        .iter()
        .map(|e| match e {
            acadrust::compound_file::CompoundEntry::Storage(c) => storage_depth(c),
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn issue_75_40000_entries_chained_through_left_siblings_do_not_overflow_the_stack() {
    let blob = big_blob(&chained_entries(68, 2), 0);
    assert!(blob.len() > 5_000_000);
    let out = bounded(move || StructuredStoragePayload::decode(&blob)).expect("bounded run");
    if let Some(cf) = out.compound_file {
        assert!(cf.root.entries.len() <= 40_000);
    }
}

#[test]
fn issue_75_40000_nested_storages_do_not_overflow_the_stack() {
    let blob = big_blob(&chained_entries(76, 1), 0);
    let out = bounded(move || StructuredStoragePayload::decode(&blob)).expect("bounded run");
    if let Some(cf) = out.compound_file {
        assert!(storage_depth(&cf.root) <= 64, "depth {}", storage_depth(&cf.root));
    }
}

#[test]
fn issue_75_entries_sharing_one_chain_do_not_each_keep_a_copy() {
    // 3000 stream entries, all pointing at one 1 MiB chain.
    let mut entries = vec![dir_entry("Root Entry", 5, END, 0)];
    put32(&mut entries[0], 76, 1);
    for i in 1..=3000usize {
        let mut e = dir_entry("s", 2, 0, 1 << 20);
        if i < 3000 {
            put32(&mut e, 68, i as u32 + 1);
        }
        entries.push(e);
    }
    let dir_sectors = entries.len().div_ceil(4);
    let mut k = 1;
    while k * 128 < k + dir_sectors + 2048 {
        k += 1;
    }
    let start = (k + dir_sectors) as u32;
    for e in entries.iter_mut().skip(1) {
        put32(e, 116, start);
    }
    let blob = big_blob(&entries, 2048);
    let n = blob.len();
    assert!(n < 1_500_000);
    let (out, peak, block) =
        bounded(move || lap(|| StructuredStoragePayload::decode(&blob))).expect("bounded run");
    assert!(peak <= few_mb(n), "peak {peak} bytes");
    assert!(block <= few_mb(n), "block {block} bytes");
    let kept: usize = out.compound_file.map_or(0, |cf| {
        cf.root
            .entries
            .iter()
            .map(|e| match e {
                acadrust::compound_file::CompoundEntry::Stream(s) => s.content.encode().len(),
                _ => 0,
            })
            .sum()
    });
    assert!(kept <= n, "kept {kept} bytes of stream data from a {n} byte blob");
}
