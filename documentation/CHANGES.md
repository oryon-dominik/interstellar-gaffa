---
## [2026.9.1](https://github.com/oryon-dominik/interstellar-gaffa/compare/2026.9.0..2026.9.1) - 2026-09-29

### Gaffa

#### Bug Fixes

- **(gaffa)** terminal restoration writes its escapes only to a terminal — a redirected log ended in ESC[?25h ESC[0m - ([d950ff1](https://github.com/oryon-dominik/interstellar-gaffa/commit/d950ff1866b418a95ee3fc405fca157880a5cd4e))

#### Chores

- **(gaffa)** changelog tags are CalVer only — v0.5.1 served the first release - ([98d3e8b](https://github.com/oryon-dominik/interstellar-gaffa/commit/98d3e8b63ba14d15dc0cb33e14b0c7738700cb79))
- **(gaffa)** the test_* ignore rule goes — it hid a manual fixture and would hide any tests/test_*.rs - ([d29c807](https://github.com/oryon-dominik/interstellar-gaffa/commit/d29c80753c1a93684952da65c827fba5417e6fe3))

#### Documentation

- **(gaffa)** drop the pre-CalVer note — 0.1.0 to 0.5.1 are yanked - ([a7e48a0](https://github.com/oryon-dominik/interstellar-gaffa/commit/a7e48a0ddcd5ae81e628a0d118fa48a257989334))
- **(gaffa)** LICENSE is the plain MIT text — the SPDX line made GitHub report it as Other - ([d6195a6](https://github.com/oryon-dominik/interstellar-gaffa/commit/d6195a6d7edfbfff1f139378fb5ea7fd48ea9468))

#### Refactor

- **(gaffa)** styling on crossterm, colored out — its MPL-2.0 would bind every binary release to ship its source - ([c867170](https://github.com/oryon-dominik/interstellar-gaffa/commit/c8671707cfffb71b746c1fb2fbcf9d2131e08ef0))
- **(gaffa)** paint keeps its rendering private — plain() and styled() served only the tests - ([e92b07c](https://github.com/oryon-dominik/interstellar-gaffa/commit/e92b07c9531ce6ea7a20408e5633dd9544d2cda6))

#### Styling

- **(gaffa)** cargo fmt over the seven lines it had marked since before 2026-09-28 - ([41aa483](https://github.com/oryon-dominik/interstellar-gaffa/commit/41aa483d9b1bba807ee35cce35ba27daf92c6eb8))

#### Testing

- **(gaffa)** the log-file test waits for the line — a fixed 2 s killed gaffa before pwsh had printed - ([5ac53d4](https://github.com/oryon-dominik/interstellar-gaffa/commit/5ac53d4857a37490ba14e0fb39362c4e790ed045))
- **(gaffa)** throwaway files go to cargo's scratch dir — the package root stays clean - ([2734b55](https://github.com/oryon-dominik/interstellar-gaffa/commit/2734b55121125910c9cb1ef7949dadc62af17094))
- **(gaffa)** the manual-testing Procfile joins the repository — the ignore rule had kept it local - ([367db16](https://github.com/oryon-dominik/interstellar-gaffa/commit/367db163358ecc35b75e6e5fb506cc3d054daaf9))
- **(gaffa)** the ignored test waiting for "Starting" goes — the banner replaced that line, the redirect test covers the output - ([e730a8a](https://github.com/oryon-dominik/interstellar-gaffa/commit/e730a8a998916185d3cf425811af789e78181461))



---
## [2026.9.0](https://github.com/oryon-dominik/interstellar-gaffa/compare/v0.5.1..2026.9.0) - 2026-09-25

### Ci

#### Bug Fixes

- **(ci)** upload only the archives and checksums — the package directory broke the release - ([cdd4a30](https://github.com/oryon-dominik/interstellar-gaffa/commit/cdd4a30fb47a8b8a8c342b5c5a320fcccc842372))



### Gaffa

#### Chores

- **(gaffa)** adopt the template MIT licence — one licence text for every FOSS package - ([83de314](https://github.com/oryon-dominik/interstellar-gaffa/commit/83de3145c25a8a33c7fea6a8498698e215562e79))

#### Continuous Integration

- **(gaffa)** publish release binaries per tag — cargo-binstall finds them without compiling - ([be4d534](https://github.com/oryon-dominik/interstellar-gaffa/commit/be4d534cc3f1b2431423cefb9e51d087c0ac9b91))
- **(gaffa)** mint CalVer releases with a changelog — the archives carry the licences of everything they link - ([024d1ff](https://github.com/oryon-dominik/interstellar-gaffa/commit/024d1ff72e2a1534e3293ec90c87a40827c27aa2))
