# libviprs/acadrust, what this fork carries on top of hakanaktt/acadrust

`main` is `hakanaktt/acadrust` main plus the patches below, nothing else. libviprs pins this repo by
git revision (`rev = "..."` in its `Cargo.toml`), never crates.io. Every patch has an upstream status,
and anything that is not listed here is a bug in the fork.

To see the difference at any time: `git diff upstream/main origin/main --stat`.

## Source patches

| Patch | Why libviprs needs it | Upstream status |
| --- | --- | --- |
| `Insert::set_scale_unchecked`, used by the DWG reader for INSERT and MINSERT (#52) | the reader reports the scale the file stores, even 0.0, instead of rewriting it to 1e-12 | not proposed yet, draft in the fork notes |
| R2004 header trailer is optional (#60) | DWG files written by LibreDWG (the oracle fixtures) stop the header before the trailing BitLongs and fail to read | not proposed yet, draft in the fork notes |

## Tests the fork adds

`tests/insert_zero_scale.rs` with `tests/data/insert-zero-scale.dwg` (#52), `tests/polyline_normal_issue_160.rs`
(the heavy POLYLINE extrusion survives the DXF writer; upstream passes it, it stays as a regression guard).

## Fork machinery waiting on a decision (#47)

`.github/workflows/ci.yml`, `Cargo.lock`, `SUITE_REV`, `tests/suite_pin_check.rs`, `tests/suite_rev_reachable.rs`,
`tools/suite_pin_check.sh`, `tests/acadsharp/README.md`: the acadsharp-rs-tests pin. The parity suite is an
experiment now, so these can go, but removing the suite pin is Rom's call.

## Dropped on purpose (taken from upstream)

The pre-2007 DXF code page read and write and the code page gate (#9, upstream #267 decodes every CIF escape),
the AC1009 DXF layout, the binary DXF integer width, the extra DWG header variables and style-name resolution
(#102, upstream #218/#221/#223 do it their way), the clockwise HATCH arc conversion (#159, upstream #242
keeps the stored angles in the model), and the MTEXT parser edits (same behaviour as upstream). libviprs
reads DWG through `DwgReader` and uses none of them. The behaviour differences are recorded in
libviprs/acadrust#56 to #59.
