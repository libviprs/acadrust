# Differential conformance against ACadSharp

ACadSharp is the reference implementation this crate is measured against. It is
a .NET library, so running it here is not on the table, and the comparison is
against a recording of it instead: [`libviprs-dep`][dep] drives the real
ACadSharp through a flattener and commits, per fixture, the canonical record
dump it produced.

[dep]: https://github.com/libviprs/libviprs-dep

```bash
scripts/fetch-acadsharp-corpus.sh
ACADSHARP_CORPUS=target/acadsharp-corpus cargo test --test acadsharp_conformance

# the human-readable version, one line per fixture
ACADSHARP_CORPUS=target/acadsharp-corpus \
  cargo test --test acadsharp_conformance -- --ignored report --nocapture
```

With no corpus the test skips rather than fails. Fetching it is a network call
and a crate's suite should not need one to go green.

## Formatting this harness: never `cargo fmt`

Format the one file you touched, with the crate's edition:

```bash
rustfmt --edition 2021 tests/viprs/mod.rs
```

`cargo fmt` is the wrong tool here and a path argument does not save you: it ignores the
argument and formats the whole crate, which rewrites seven `src/` files that have never
been rustfmt-clean — `current_transparency.rs`, `nested_copy.rs`, `entities/acis/types.rs`,
`entities/translate.rs`, `io/dxf/reader/section_reader.rs`, `io/dxf/writer/section_writer.rs`
and `lib.rs`, whose `mod` order it also sorts. All seven are upstream's files at commits
this fork has in common with it, so reformatting them buys nothing here and costs a
conflict on every future rebase. That is why they are left alone rather than fixed.

`cargo fmt --check` is safe and is red on those same seven files. `rustfmt --check` on one
file is the version worth believing.

## Where it stands

52 fixtures, of which 45 carry a record dump, read from libviprs-dep at the pinned commit.

| Verdict | Count |
| --- | --- |
| `MATCH` | 42 |
| `DIFF` | 2 |
| `COUNT` | 0 |
| `UNCOMPARED` | 1 |
| no recording | 7 |

The seven with no dump are not a gap in the corpus and nothing here should try to fill
it: five are checked by the scenario capture (`expectations/g13_scenarios.json`) and two by
the benchmark capture (`benchmarks/amplification.json`), which is why libviprs-dep's own
`recorded_fixtures()` reads all three artefacts rather than the dumps alone. They are
limit, refusal and block-amplification fixtures, so a geometry dump is not what records
them. The 45 are the whole of the geometry oracle.

The harness models the DWG format's own semantics: the arbitrary axis algorithm, block
expansion including an insertion's own extrusion, reflection, non-uniform scale, dimension
blocks, a TABLE's cached block, an insertion's ATTRIBs, and MLINE's element offsets. It was
bootstrapped rather than trusted: block expansion reproduces `g13_insert` exactly, rotation
and the `flags` bit included, against a recording made by the real ACadSharp.

Both real drawings produce all 380 records and `g13_mline` all 21, and no fixture disagrees
on a count any more. What is left is one record in each real drawing, and in both of them
acadrust is the side that is right, which is why they have a section of their own below.

## Three differences that were the harness's own

Of the differences this report used to carry, three belonged to this harness rather than to
either reader. That is the outcome a differential test should expect to find first, and the
reason they are written down after being fixed is so the next reader does not re-open them.

**The SOLID arm emitted the raw extrusion.** A record's `normal` is the placed plane as a
unit vector; the SOLID arm handed back the entity's extrusion as the file stores it, so
`g13_solid` reported `(1,2,2)` where the recording has `(0.333,0.667,0.667)` — the same
direction over its length, with every coordinate beside it agreeing to six decimals. What
proved the harness owned it was `g13_leader`: handle `4C` carries that same `(1,2,2)`
extrusion, its record spells it `(0.333333,0.666667,0.666667)`, and the fixture MATCHed.
One arm of one harness normalised and another did not.

**The harness emitted values that are not finite.** `g13_nan_bulge` holds a NaN bulge and
an infinity and the harness put both on the wire. libviprs-dep's `docs/WIRE.md:386` forbids
a producer from emitting either in a geometry record: the entity is not emitted at all and
a `NON_FINITE_GEOMETRY` warning naming its handle goes out instead, which is why the
recording holds no record for handles `49` and `4A` and the verdict was `COUNT:2/0`. The
empty recording was right and the difference in count was the harness's. Warnings are not
part of the comparison, so the fixture now agrees on two empty record sets.

**`Record::parse` collapsed whitespace inside a recorded value.** It tokenised the
recording's line on whitespace and rejoined it with single spaces, which is harmless for
`handle=` and `flags=` and wrong for the text that follows: handle `576` in both real
drawings carries a run of eleven spaces, the recording spells it out, acadrust reads it
back, and the harness ate it on the recording's side alone. A difference neither reader had.

## Two defects the harness found in acadrust

**The DWG MLINE closed flag was read and never assigned.** `g13_mline`'s fourth multiline
is written with `MLineFlags.Has | MLineFlags.Closed` and the recording says `closed=1` for
its three element polylines. The DWG reader took the `Openclosed` short off the stream and
then dropped it, so the entity came back carrying `HAS_VERTICES` alone and
`MLine::is_closed()` answered false after a DWG load. Every coordinate on all 21 records
agreed, which is what made it a flag that is not read rather than a path that is wrong. The
DXF reader and the DWG writer were both already right, and the round-trip tests missed it
for a plain reason: none of them closes an MLINE, so the flag was zero on both sides of
every comparison they make.

**MIF `\U+XXXX` escapes were decoded without asking the code page.** `real_AC1018` is an
ANSI_1252 drawing, and in it `94\U+00B0` is eight characters of MTEXT content rather than a
transport escape: `U+00B0` is representable in ANSI_1252, so nothing about that character
needed escaping and the escape in the file is text. acadrust decoded it anyway and returned
`94°` on handles `634` and `63E`. The encoder only ever escapes a character the drawing's
code page cannot represent, so the decoder now tests the same condition and decodes only
what that condition could have written.

## Where the harness deliberately stops

One lowering is the adapter's product decision rather than the format's semantics, and this
harness does not reimplement it:

- **A hatch loop carrying a curve** (`g13_hatch`). ACadSharp emits a warning and then the
  loop's edges as records of their own. Which curve becomes which record is a choice, not a
  fact about the file. Straight-edged loops ARE compared, and match.

That line is the whole reason this report is worth reading. Past it the harness would be a
second implementation of the thing it is measuring, and every difference it found could be
its own. Where it cannot express a fixture it says `UNCOMPARED` rather than emitting fewer
records, because a short count reads as the other side's defect.

MLINE used to sit beside it, on the grounds that mitring is a choice. That reading did not
survive libviprs-dep#110: the joint is not chosen, it is `t = effective / dot(miter, side)`
from the per-vertex miter bisector the file carries, which upstream took from the ODA
specification and confirmed against `real_AC1032` two independent ways. The style lookup it
needs resolves inside the same document. So it is reproduced here, and with the closed flag
read `g13_mline` agrees on all 21 records.

## Where acadrust is ahead of the recording

Two records are left, and in both of them the recording is the side that is wrong. Each is
worth reading before it is touched, because each has an obvious change that turns the
verdict green by throwing data away.

**`real_AC1018` handle `64B`: a MIF escape that is genuine transport.** The drawing stores
`\U+220545,6` and acadrust returns `∅45,6`. `U+2205` is not representable in the drawing's
ANSI_1252 code page, so the escape is the only way that character crosses and decoding it
is the correct read — the same condition as the section above, on the other side of it.
ACadSharp implements no MIF decoding at all: `ReadVariableText` in
`DwgStreamReaderBase.cs:880-891` reads the string, strips NULs and hands it back. And the
recording refutes itself. `real_AC1032` is the R2018 save of the same drawing, and there the
same reference implementation returns the literal `∅45,6` for this same handle `64B`. Any
change that makes this record match is a regression. The trap is that the change is a
one-liner: deleting the decode flips this record and, from the state this report used to
describe, the two `\U+00B0` records with it — three records green at once, which reads as a
win and is one correct fix beside a silent loss of a character the drawing cannot carry any
other way.

**`real_AC1032` handle `79D`: a multiline ATTRIB's text.** An R2018 multiline attribute
carries its real value in an embedded MTEXT and the single-line field ahead of it holds only
the first line, so acadrust lifts the text out of the embedded MTEXT — in
`src/io/dwg/dwg_stream_readers/object_reader/entities.rs:4364-4387`, gated on `att_type > 1`
and overriding only when the embedded value is non-empty — and returns `my multi line text
for the attrrib`. ACadSharp returns `""`, and is contradicted by its own `real_AC1018`
recording, which carries that string for this same handle. acadrust's writer is symmetric:
`src/io/dwg/dwg_stream_writers/object_writer/entities.rs:1286-1298` writes the embedded
MTEXT back under the same `att_type > 1` gate. And `dwg_document_builder.rs` derives
`e.line_count` from `e.value` in the ATTRIB arm, so an empty value would also report the
wrong line count. Matching the recording here would be a data-loss regression.

Neither fixture is marked `UNCOMPARED`. That verdict says the harness cannot express a
fixture, and reaching for it here would stop comparing the other 379 records of a real
drawing in order to hide one record the harness gets right — which is how a real regression
arrives unseen. The baseline exists so that a justified `DIFF:n` stays policed: the number
is recorded, a second record disagreeing fails the test, and the reason the number is not
zero is this section.

Two matching decisions are worth naming because they look like omissions here:

- **A nested INSERT inside a dimension block is not walked**, because the reference
  implementation does not walk one: "a dimension block is generated geometry, not a user
  block". That skip is 12 records in each real drawing — `_BoxBlank` twice inside `*D8`,
  `_ArchTick` twice inside `*D4` — which this harness used to emit and the recording never
  had.
- **A TABLE is expanded as an insertion**, because `TableEntity` derives from `Insert`
  upstream and the flattener dispatches on that base type. That block is 56 of each real
  drawing's records: 31 cell borders, 24 cell texts and one background polygon.

## Why a baseline instead of asserting equality

`BASELINE.tsv` records the per-fixture verdict and the test asserts against it.
acadrust is not ACadSharp and this going green was never the goal: a
conformance test that is red on arrival is one nobody can use twice, because
the second run tells you exactly what the first did.

A fixture that gets worse fails and names what moved. A fixture that gets
better also fails, and asks you to re-record. Both are the point.

```bash
ACADSHARP_CORPUS=... cargo test --test acadsharp_conformance -- --ignored record_baseline
```

## The pin

`CORPUS.pin` names the libviprs-dep commit the corpus is read from. It is
pinned rather than floating for the reason the corpus is committed over there
in the first place: a DWG header carries creation and update timestamps, so a
regenerated fixture is a different file with a different digest, and a
comparison against whatever `main` holds today cannot be reproduced.

## Licensing

The corpus is MIT. The generated fixtures are libviprs-dep's own, written by
ACadSharp's `DwgWriter`; `real_AC1018.dwg` and `real_AC1032.dwg` are ACadSharp's
own sample drawings. Nothing from the corpus is vendored into this repository,
which keeps provenance in the one place that records it.
