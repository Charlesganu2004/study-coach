# Dependency intake audit

Reviewed November 1, 2026, before compiling these dependencies. Scope: the exact cached crates.io packages listed below, with `serde_json = "=1.0.154"` and default features. No package build script, library, example, test, or shipped executable was executed during this review. No external repository was cloned and no antivirus exclusion was created.

**Verdict: PASS-WITH-NOTE for the exact versions reviewed.** No blocking finding was identified by the checks performed. This is a bounded static review, not a guarantee that the packages contain no vulnerabilities. Any additional resolved version or enabled feature needs a fresh review before use.

## Provenance and versions

Each cached `.crate` archive's SHA-256 matched the version checksum returned by `https://crates.io/api/v1/crates/NAME/VERSION`. The archive's files were compared byte-for-byte against the local registry extraction: 240 files across six archives, zero mismatches. Archive paths were checked for parent traversal and links. No downloads were needed.

| Package | Version | License declared in package | Minimum Rust | Canonical repository | Published UTC |
|---|---|---|---|---|---|
| serde_json | 1.0.154 | MIT OR Apache-2.0 | 1.71 | https://github.com/serde-rs/json | 2026-10-11 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | 1.56 | https://github.com/serde-rs/serde | 2026-07-18 |
| serde | 1.0.229 | MIT OR Apache-2.0 | 1.56 | https://github.com/serde-rs/serde | 2026-07-18 |
| itoa | 1.0.18 | MIT OR Apache-2.0 | 1.68 | https://github.com/dtolnay/itoa | 2026-03-20 |
| memchr | 2.8.3 | Unlicense OR MIT | 1.61 | https://github.com/BurntSushi/memchr | 2026-07-08 |
| zmij | 1.0.23 | MIT | 1.71 | https://github.com/dtolnay/zmij | 2026-07-13 |

The upstream commit identities below came from `.cargo_vcs_info.json` inside each checksum-verified archive. Registry archives can contain packaging changes from the upstream tree; the archive checksum is the exact artifact identity used for this audit.

| Package | Archive SHA-256 | Upstream commit |
|---|---|---|
| serde_json 1.0.154 | e7e9cc8b1b85264074fbcc02a88680c4096b1e47df8f739dceb03bf482f04bd6 | 20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8 |
| serde_core 1.0.229 | 67dca2c9c51e58a4791a4b1ed58308b39c64224d349a935ab5039aa360942a48 | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 |
| serde 1.0.229 | 4148590afebada386688f18773da617792bf2ef03ffc1e4cbd2b1d45b023e0ba | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 |
| itoa 1.0.18 | 8f42a60cbdf9a97f5d2305f08a87dc4e09308d1276d28c869c684d7777685682 | af77385d0daf4d0e949e81f2588be2e44f69f086 |
| memchr 2.8.3 | cf8baf1c55e62ffcace7a9f06f4bd9cd3f0c4beb022d3b367256b91b87513d98 | 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d |
| zmij 1.0.23 | 29666d0abbfad1e3dc4dcf6144730dd3a3ab225bbbdac83319345b1b44ccfc1b | 7b7cc48b58028e8af7be87e94c0c1c8936f1a57c |

GitHub metadata checks returned the exact canonical names above, with `fork=false`, `archived=false`, and `disabled=false`. Latest reported upstream pushes: serde-rs/json 2026-10-16; serde-rs/serde 2026-10-16; dtolnay/itoa 2026-06-24; BurntSushi/memchr 2026-08-10; dtolnay/zmij 2026-08-06. These are maintenance signals, not a code-safety proof.

## Dependency and execution boundaries

The default active dependency path is `serde_json` using `itoa`, `memchr`, `serde_core`, and `zmij`. No `derive`, `preserve_order`, `raw_value`, `arbitrary_precision`, or optional `no-panic` feature is requested. Upstream development dependencies are not this application's dependencies.

`serde_json` also declares a `serde` dependency under `cfg(any())`, and `serde_core` declares `serde_derive` under that always-false target. Cargo can include target-inactive packages and their dependencies in lockfile resolution. They must not be confused with compiled dependencies. This receipt includes `serde` because it was already cached; any additional package selected during lockfile generation requires its own review if it will be built.

Build scripts were read in full:

- `serde_json/build.rs`: reads target architecture and pointer width, then emits Cargo configuration directives. No command execution or network access.
- `serde_core/build.rs` and `serde/build.rs`: generate a fixed `private.rs` module under Cargo's `OUT_DIR`, invoke Cargo-provided `RUSTC --version`, and emit compiler-compatibility configuration. The command is the configured compiler, with a constant `--version` argument, not a downloaded executable or shell string.
- `zmij/build.rs`: invokes Cargo-provided `RUSTC --version`, reads optimization settings, and emits compiler configuration. No network access.
- `itoa` and `memchr`: manifests declare `build = false`.

This review assumes the locally configured Rust compiler and Cargo environment are trusted. Rust build scripts are not a security sandbox.

## Static findings

- The archive scan found zero bidi controls, zero selected invisible formatting characters, zero executable binary signatures or executable-library extensions, zero archive traversal paths, and zero archive links.
- A source search for network clients, download-and-execute commands, shell execution, known credential prefixes, and private-key headers found compiler-version commands described above. `serde_json` additionally contains TCP documentation and localhost tests; those are not network operations performed by the JSON parsing API.
- Packaged CI workflows in serde_json, itoa, and zmij were inspected. They use read-only repository permissions and normal `pull_request` triggers. No `pull_request_target` trigger or interpolation of untrusted issue/PR text into shell commands was found. Actions and reusable workflows use mutable tags/branches; this is noted upstream CI risk. These workflows are not imported into or executed by this project.
- No Git hook directory was present in the reviewed archives. No SQL construction or permission-changing installation script was identified by the pattern search.
- No known secret pattern matched the search. A pattern search cannot establish the absence of all secrets or malicious behavior.

## Vulnerability lookup and tools

An HTTPS POST to `https://api.osv.dev/v1/querybatch` queried ecosystem `crates.io` for each of the six exact package/version pairs. The response reported **zero advisories for each pair**. Only package names and versions were sent, never repository source, credentials, or private documents. Advisory databases can change; recheck before updating or submitting later versions.

`gitleaks`, `trivy`, `osv-scanner`, and `semgrep` executables were not installed. They were not silently treated as passing scans. Checks used the OSV API, canonical metadata, archive hashes, local source comparison, targeted `rg` searches, and manual manifest/build-script/workflow inspection. No dynamic malware analysis or full semantic code audit was performed.

`Get-MpComputerStatus` reported `AntivirusEnabled=True` and `RealTimeProtectionEnabled=True`. Defender settings were not changed. This is a protection-status check, not a clean malware-scan result.

The local registry extraction did not contain `.cargo-checksum.json` files. That missing metadata was not treated as evidence of integrity; archive checksums against crates.io plus direct archive/source comparisons supplied the integrity check instead.

## Reproduction

1. Obtain each version's canonical metadata from `https://crates.io/api/v1/crates/NAME/VERSION`; compare `version.checksum` against the local `.crate` SHA-256.
2. Read the tar archive without executing or importing anything. Reject traversal paths and links; compare every archived file with the cached extraction.
3. Inspect package manifests, all `build.rs` files, and packaged CI workflows. Search sources for fetch-and-execute patterns, subprocesses, secret formats, hidden Unicode, binaries, SQL construction, and permissions changes.
4. Query OSV with `{"package":{"name":"NAME","ecosystem":"crates.io"},"version":"VERSION"}` for every exact version. Record actual advisories rather than assuming an empty result for failures.
5. Confirm Cargo's resolved graph and enabled features before compilation. Re-audit versions or features beyond this receipt.

## Target-inactive lockfile packages

Offline lockfile generation additionally resolved the five packages below through the always-false target dependency. This extension covers registry metadata, archive integrity, source comparison, and OSV queries only. It does not authorize executing these packages or enabling `serde_derive`. Confirm they are absent from `cargo tree --offline --locked -e normal,build` before building the default application.

| Package | Version | License | Rust | Archive SHA-256 | Upstream commit |
|---|---|---|---|---|---|
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | 1.71 | 985e7ec9bb745e6ce6535b544d84d6cd6f7ad8bd711c398938ae983b91a766d9 | ed8a5497669cd63db33bf24646f261b012bbbc4a |
| quote | 1.0.47 | MIT OR Apache-2.0 | 1.71 | 1fbf4db142a473a8d80c26bbf18454ed458bf8d26c8219c331daecfdbd079001 | 723dcb47d3f0ddc896e17287c8a8d3f2ea2317d5 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | 1.71 | e7a5d71263a5a7d47b41f6b3f06ba276f10cc18b0931f1799f710578e2309348 | 7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8 |
| syn | 3.0.9 | MIT OR Apache-2.0 | 1.71 | d78c8dee4c7bf0e14673097256fed6142ce9d3b85a408189d07482442145823b | 7c44a5565b4421035101cc16184e9af08ab5fc77 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1.71 | a2c754d6c33795a1c324727428e5a7dedb5b06195f9890bdbcba760d3e246563 | c1ac3afb721f3cea6527bc2255340a04d0de3ca0 |

All five archives matched canonical crates.io checksums; their 234 archived files matched cached sources with zero mismatches. No archive traversal paths or links were found. OSV returned zero advisories for each exact version. `proc-macro2` and `quote` contain build scripts that were not executed and are outside the active-build clearance above.

Canonical repositories reported by crates.io are https://github.com/dtolnay/proc-macro2, https://github.com/dtolnay/quote, https://github.com/serde-rs/serde, https://github.com/dtolnay/syn, and https://github.com/dtolnay/unicode-ident. GitHub returned matching names, `fork=false`, `archived=false`, and `disabled=false`; additional upstream pushes were dated 2026-08-25, 2026-10-25, 2026-10-30, and 2026-10-30 for proc-macro2, quote, syn, and unicode-ident respectively. All five crate versions were reported as not yanked.

Skill used: `dep-audit`, loaded from `C:\Users\Charl\.codex\skills\dep-audit\SKILL.md`.
