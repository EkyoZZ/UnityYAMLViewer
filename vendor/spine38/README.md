# Spine 3.8 runtime source

Vendored from https://github.com/jabuwu/rusty_spine3.8 at commit
`4ff8b7280aeb7367f235752c30dc85184c01452a` (retrieved 2026-10-11).
The runtime is a Rust translation of spine-c with Rust wrappers, not the
Spine 4.1/4.2 runtime published under the same crate name on crates.io.
The generated parser rejects Spine 3.8.75 exports; this upstream behavior is preserved.

Local packaging uses only the source tree and LICENSE, a minimal Cargo manifest,
and the portable allocator/string implementation (the upstream libc option
does not compile on Windows). No upstream examples or image assets are bundled.
The upstream source retains its original comments, including some outdated
references to newer Spine versions and legacy compiler warnings.

See [LICENSE](LICENSE) for the Spine Runtimes, wrapper and BSD licensing terms.
Integration and distribution must comply with the Spine Runtimes License Agreement:
https://esotericsoftware.com/spine-runtimes-license
