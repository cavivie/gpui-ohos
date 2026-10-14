# Direct-window experiment artifacts

Exploratory, OHOS-only patches for wgpu `8b419de0906f37af69d728f1735132e427a3436f`
and gpui-ohos `9ea8bc5` plus the lazy path-target change. These are archived
experimental inputs, not production dependency patches. Both pass `git apply
--check` against their respective local baselines. No branch applies them.

`NSDIRECT=1` compiles direct default-framebuffer rendering; `NSDIRECT=0` uses the
original offscreen framebuffer and Y-flipping blit. The gpui-ohos shader/global
change and HAL change must be used together. Ordinary offscreen path passes
keep their original orientation. The normal renderer still clears the entire
frame and uses FIFO. No panel rate, resolution or AA changes are part of this
experiment.

The HAL patch includes a one-time `NSDIRECT_VERIFY` diagnostic added only for the
subsequent debug path probes. The timing HAPs predate that diagnostic; it does
not contribute to the A/B timing results. The GPUI patch omits synthetic scenes
and profiler instrumentation. A device framebuffer query after submission is
not a complete render-pass or driver validation.

The raw surface handle is valid only while its acquired wgpu surface texture is
alive. The prototype explicitly rejects a single submission spanning multiple
native windows. It has not been qualified for arbitrary imported textures,
surface copies/readback, multiwindow submission, resizing, context loss, or the
full WebGPU CTS. Both first GL error probes returned 1280; see the report.

Timing, source snapshots, build scripts, HAPs, native artifacts and the CPU trace
were kept in a local capture directory, not in this repository. The source copy
uses local paths for all ten locked wgpu workspace packages; non-wgpu lockfile
packages and versions are unchanged. `normal-source/` contains the timed renderer source;
`project/` ends on the debug synthetic-path variant. Do not rebuild the normal
benchmark from that final directory without restoring the normal source.

See [the experiment report](../../direct-window-experiment-2026-10-13.md).
