# Parity against ACadSharp lives in the suite

This directory used to hold a second copy of the flattener, a comparator and a
verdict file (`tests/viprs/mod.rs`, `tests/acadsharp_conformance.rs`,
`BASELINE.tsv`), measured against libviprs-dep's six-decimal `.txt` recording.
All three now live in
[`acadsharp-rs-tests`](https://github.com/libviprs/acadsharp-rs-tests), which
runs this crate, `acadsharp-rs` and the .NET oracle over one corpus with one
comparator and writes one verdict with a reason on every row that is not a
match. The fork is a consumer of it: `SUITE_REV` pins the suite commit, and the
`Suite (acadsharp-rs-tests)` job in `.github/workflows/ci.yml` runs the suite's
gate (`tools/run-tests.sh --in-ci`) against this tree.

The verdicts are in the suite's
[`parity/SUMMARY.md`](https://github.com/libviprs/acadsharp-rs-tests/blob/main/parity/SUMMARY.md).
The two places acadrust reads what the drawing holds and the recording does not
(a MIF escape, libviprs-dep#111, and a multiline attribute's text, #112) are
`KNOWN:1` rows there, with the arguments in their `reason`, and are policed from
there: a row whose divergence goes away is red, and so is one that moves.

To run it locally, from the suite with this tree in its slot:

```bash
tools/run-tests.sh --acadrust /path/to/this/checkout
```

## Formatting: never `cargo fmt`

Format the one file you touched, with the crate's edition:

```bash
rustfmt --edition 2021 path/to/the/file.rs
```

`cargo fmt` is the wrong tool here and a path argument does not save you: it ignores the
argument and formats the whole crate, which rewrites fourteen files that have never been
rustfmt-clean. `cargo fmt --check` names them: twelve under `src/`:
`current_transparency.rs`, `document.rs`, `entities/acis/types.rs`,
`entities/translate.rs`, `io/dwg/dwg_reader.rs`,
`io/dwg/dwg_stream_writers/object_writer/entities.rs`,
`io/dwg/dwg_stream_writers/object_writer/mod.rs`, `io/dxf/reader/section_reader.rs`,
`io/dxf/writer/section_writer.rs`, `lib.rs`, whose `mod` order it also sorts,
`nested_copy.rs` and `objects/mod.rs`; and two under `tests/`:
`dxf_autocad_conformance.rs` and `read_visiting.rs`. All fourteen are upstream's files at
commits this fork has in common with it, so reformatting them buys nothing here and costs
a conflict on every future rebase. That is why they are left alone rather than fixed.

`cargo fmt --check` is safe and is red on those same fourteen files. `rustfmt --check` on
one file is the version worth believing.
