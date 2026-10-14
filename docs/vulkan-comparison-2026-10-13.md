# Vulkan comparison on the current renderer, 2026-10-13

Switching this device to Vulkan produced a small reduction in CPU draw-through-
present duration, but did not demonstrate better long-frame behavior. Keep GLES
as the default on the evidence from this comparison.

## Revisions and method

- NearSend: `aa8b0a36253a8135bfbc63d7f3015f575b4c6860`.
- gpui-ohos: `9ea8bc50f9dd3a03fcdf4622bddf591c4ab9e578`, including integral-mask
  geometry clipping and coalesced scene instance uploads.
- Ability: `eab9d11ad00ab8ab85aff5e5c3cfa86cdd6fb5bb`.
- GPUI: `ohos-rs/zed@f45c7c22d0`; wgpu:
  `8b419de0906f37af69d728f1735132e427a3436f`.
- Device: ALN-AL00 / Maleoon 910, 1260 x 2720, the same connected 120 Hz phone.
- Release Rust and HAP, identical source and diagnostics. Both binaries compile
  `gles`, `vulkan`, and `gpui/profiler`; a temporary compile-time `NSBENCH_BACKEND`
  selection restricts the instance to exactly one backend. Startup logs verify
  the actual selected adapter in every trial.
- Identical FIFO mode, maximum frame latency 2, shaders, instance transport,
  resolution, and frame scheduler. No XComponent frame-demand changes.
- Existing capability-based selection chooses BGRA8/Opaque on GLES and
  RGBA8/Inherit on Vulkan. These are the configurations selected by the current
  code, not artificially forced matching formats.
- Order: GLES A, Vulkan A, GLES B, Vulkan B. Each trial starts a fresh process,
  warms settings with six alternating swipes, returns to the top, and measures
  twelve alternating swipes with identical coordinates and speed.
- Hilog is streamed so the bounded device log ring cannot discard the beginning
  of a run. All collected draw records pair with present records. Renderer stages
  are sampled once per sixteen frames. No CPU sampler runs during timing.
- Battery temperature before/after each trial: 34/34, 35/35, 35/35, 35/36 C.
  This observation is not a GPU junction temperature measurement or proof of
  fixed CPU/GPU clocks.

Enabling Vulkan adds seven lockfile entries; no existing dependency identities
or versions change. Builds and packaging use isolated snapshots. The ordinary
NearSend checkout and its signing configuration are preserved.

## Results

Pooled values come from individual samples, not averages of trial percentiles.
Times are milliseconds; smaller is better.

| Metric | GLES | Vulkan | Vulkan change |
| --- | ---: | ---: | ---: |
| Paired draw/present samples | 6328 | 6284 | |
| GPUI draw median | 3.552 | 3.376 | |
| Platform draw/present median | 1.813 | 1.680 | -7.3% |
| Platform draw/present mean | 1.833 | 1.824 | -0.5% |
| Platform draw/present P95 | 2.227 | 2.515 | +12.9% |
| Platform draw/present P99 | 2.527 | 2.715 | +7.4% |
| Draw start through present return median | 5.409 | 5.213 | -3.6% |
| Draw start through present return mean | 5.430 | 5.245 | -3.4% |
| Draw start through present return P95 | 6.395 | 6.297 | -1.5% |
| Draw start through present return P99 | 6.890 | 6.779 | -1.6% |
| Draw start through present return maximum | 9.642 | 9.564 | |
| Samples exceeding 8.333 ms | 6 (0.095%) | 11 (0.175%) | |
| Active draw-start interval median | 8.283 | 8.283 | |

Per-trial draw-start-through-present-return measurements:

| Trial | Samples | Median | P95 | P99 | >8.333 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| GLES A | 3163 | 5.423 | 6.420 | 6.922 | 4 |
| Vulkan A | 3141 | 5.218 | 6.299 | 6.853 | 5 |
| GLES B | 3165 | 5.395 | 6.363 | 6.861 | 2 |
| Vulkan B | 3143 | 5.208 | 6.282 | 6.722 | 6 |

The rare over-budget counts do not establish a statistically reliable increase
in display drops. They show that this test has not demonstrated elimination or
reduction of long frames. CPU call-return durations and draw-start cadence do
not measure GPU completion, compositor scheduling, or panel scanout, and do not
establish uninterrupted 120 Hz display updates.

## Where the time goes

| Sampled renderer stage | GLES median / P95 | Vulkan median / P95 |
| --- | ---: | ---: |
| Acquire surface texture | 0.013 / 0.022 | 0.411 / 0.533 |
| Queue submission | 0.448 / 0.658 | 0.441 / 1.314 |
| Present call | 1.078 / 1.451 | 0.466 / 0.600 |

Vulkan avoids the GLES presentation blit and returns from present sooner, but
texture acquisition costs more and queue submission has a longer tail. The
platform-stage mean is essentially unchanged. Stage percentiles are not
additive, and faster present return is not an exclusive measurement of saved
blit work.

GPUI draw time also differs despite unchanged code. Therefore the entire
0.196 ms reduction in whole-frame median cannot be attributed specifically to
removing the GLES copy. These two backend pairs provide device evidence, not a
guarantee for other pages, thermal conditions, or drivers. October 12 tests used
older code; their absolute durations must not be compared directly with these.

## Verification and artifacts

- Both ARM64 release builds completed with the same locked dependency graph.
- Both HAP signatures validate, use the existing device certificate, and have
  `debug=false`. Their allocated native ELF sections match the respective fresh
  builds. Logs confirm each backend and surface configuration.
- Both backends completed navigation and all measured gestures without process
  restarts. Settings screenshots match exactly across 3,074,400 pixels in the
  application region `(0, 140, 1260, 2580)`, excluding system bars. This checks
  that scene, not exhaustive Vulkan visual compatibility.
- Scripts, raw logs, source hashes, temperature observations, package checks,
  per-trial results, and pooled `comparison.json` were kept in a local capture
  directory and are not archived in this repository.
- Diagnostic GLES HAP SHA-256:
  `a0dfe120343d48d1cd89e0e531cc811186069819c043385ee1092b53a24c2373`.
- Diagnostic Vulkan HAP SHA-256:
  `30427778108f667cda3d294fcfa7a908ead3183927cacb1bad8d54e4c9a08138`.

The clean GLES release was restored after the comparison:
`nearsend/dist/NearSend-1.2.2-render-perf-release.hap`, SHA-256
`7523a7a5d50892644445e29725dc6b50ecd7d4db8c4a547b327badd2316f3468`.
Installation, absence of diagnostic markers, and navigation/scroll smoke checks
passed (PID 39755). Evidence is in `release-installation.json` and screenshots
in the capture directory. Production backend selection and rendering code are
unchanged by this experiment.
