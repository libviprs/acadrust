# libviprs/acadrust, what this fork carries on top of hakanaktt/acadrust

`main` is `hakanaktt/acadrust` main plus the patches below, nothing else. libviprs pins this repo by
git revision (`rev = "..."` in its `Cargo.toml`), never crates.io. Every patch has an upstream status,
and anything that is not listed here is a bug in the fork.

To see the difference at any time: `git diff upstream/main origin/main --stat`.

## Source patches

| Patch | Why libviprs needs it | Upstream status |
| --- | --- | --- |
| `Insert::set_scale_unchecked` in `src/entities/insert.rs`, used by the DWG reader for INSERT and MINSERT in `src/io/dwg/dwg_document_builder.rs` (#52) | the reader reports the scale the file stores, even 0.0, instead of rewriting it to 1e-12 | proposed upstream: hakanaktt/acadrust#352 (issue), hakanaktt/acadrust#354 (PR), open |
| R2004 header trailer is optional, `src/io/dwg/dwg_stream_readers/header_reader.rs` (#60, merged as #67, the rev libviprs pins) | DWG files written by LibreDWG (the oracle fixtures) stop the header before the trailing BitLongs and fail to read | proposed upstream: hakanaktt/acadrust#353 (issue), hakanaktt/acadrust#355 (PR), open |

## Tests the fork adds

`tests/insert_zero_scale.rs` with `tests/data/insert-zero-scale.dwg` (#52), `tests/issue_60_truncated_r2004_header_trailer.rs` (#60), `tests/polyline_normal_issue_160.rs`
(the heavy POLYLINE extrusion survives the DXF writer; upstream passes it, it stays as a regression guard).

## Fork machinery

None. The CI workflow, `.gitignore`, `AGENTS.md`, `CONTRIBUTING.md`, `CHANGELOG.md` and the templates are upstream's, `Cargo.lock`
is untracked like upstream, and the acadsharp-rs-tests suite pin (`SUITE_REV`, its two tests and the check script) is gone (#47).
The fork no longer asserts a parity-suite revision; the suite pins this repo itself.

## Dropped on purpose (taken from upstream)

The pre-2007 DXF code page read and write and the code page gate (#9, upstream #267 decodes every CIF escape),
the AC1009 DXF layout, the binary DXF integer width, the extra DWG header variables and style-name resolution
(#102, upstream #218/#221/#223 do it their way), the clockwise HATCH arc conversion (#159, upstream #242
keeps the stored angles in the model), and the MTEXT parser edits (same behaviour as upstream). libviprs
reads DWG through `DwgReader` and uses none of them. The behaviour differences are recorded in
libviprs/acadrust#56 to #59.
