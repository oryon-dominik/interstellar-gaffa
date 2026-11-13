---
## [2026.11.1](https://github.com/oryon-dominik/interstellar-gaffa/compare/2026.11.0..2026.11.1) - 2026-11-13

### Cli

#### Bug Fixes

- **(cli)** a failed run reports its error once — a spawn failure printed three times - ([9221049](https://github.com/oryon-dominik/interstellar-gaffa/commit/922104946584785c6ee85104687d48f893aaedb4))
- **(cli)** the banner names Ctrl+C alone — without the UI nothing reads q - ([c767d58](https://github.com/oryon-dominik/interstellar-gaffa/commit/c767d588f7a69f39b8b94106fb8b9246b7c29af4))

#### Refactor

- **(cli)** unreachable help, unused values and bare exit codes go — clap owns the help, the constants name the codes - ([3cc7107](https://github.com/oryon-dominik/interstellar-gaffa/commit/3cc7107909212ae618303ecc3762ab6295542026))

#### Testing

- **(cli)** the process-selection test checks what ran — it killed gaffa after 2 s and asserted nothing - ([9cf641b](https://github.com/oryon-dominik/interstellar-gaffa/commit/9cf641be523f6a6f6b02a738126177d9a933a21a))
- **(cli)** integration tests run the built gaffa binary in cargo's scratch dir — cargo run rebuilt per test and the system temp dir was shared - ([a3fcbef](https://github.com/oryon-dominik/interstellar-gaffa/commit/a3fcbefef0062f6bf0fc415dd5544f8d8c66c3e3))



### Deps

#### Build

- **(deps)** the clipboard is read through arboard — cli-clipboard's wl-clipboard-rs 0.7 no longer builds on Rust 1.100 - ([ded1c19](https://github.com/oryon-dominik/interstellar-gaffa/commit/ded1c19a2c7dcd61f0d8727c4bbf8d82d2d4c370))
- **(deps)** shell-words is gone — Procfile lines run verbatim through the shell since the shell-exec change - ([e14637c](https://github.com/oryon-dominik/interstellar-gaffa/commit/e14637c4055b4688a8a6b85c3f44347481e3c430))



### Errors

#### Bug Fixes

- **(errors)** [**breaking**] env file, env argument and log file fail under their own names — they were reported as Procfile errors - ([aa9da0c](https://github.com/oryon-dominik/interstellar-gaffa/commit/aa9da0ce74d89b017fdb06bc1d6e748af9316ef9))



### Lint

#### Styling

- **(lint)** two imports Linux never uses are gone — clippy -D warnings failed there - ([a972c5c](https://github.com/oryon-dominik/interstellar-gaffa/commit/a972c5c476fd15cc1a90f2207cca7e3d8ea7a7a9))



### Log

#### Bug Fixes

- **(log)** the log file takes process output in interactive mode too — it stayed empty under the UI - ([af556d8](https://github.com/oryon-dominik/interstellar-gaffa/commit/af556d8e8dd188b071d5d1950282dc1d81120df9))
- **(log)** [**breaking**] the log file takes gaffa's own messages in interactive mode — only process output reached it - ([6b620af](https://github.com/oryon-dominik/interstellar-gaffa/commit/6b620afadec1c7065d8e23192605f16efe308780))



### Manager

#### Refactor

- **(manager)** [**breaking**] children_count alone counts the live children — monitor_handle_count returned the same number - ([b722168](https://github.com/oryon-dominik/interstellar-gaffa/commit/b7221689eab0846bead4fa23730622ada7abbab6))



### Output

#### Bug Fixes

- **(output)** a lone escape drops the whole character after it — a multibyte one panicked the output handler - ([b54f2de](https://github.com/oryon-dominik/interstellar-gaffa/commit/b54f2de383ca01f90f8629c46dc05a94994db9a7))
- **(output)** stopping everything is announced as such — "Interrupt received" showed on Quit and "stop all" too - ([301869e](https://github.com/oryon-dominik/interstellar-gaffa/commit/301869e85434fe02fed8bbc0a20d59ad115c1288))

#### Refactor

- **(output)** one escape-sequence scanner serves sanitize, wrap and strip — the same CSI/OSC loop stood there three times - ([7ed3bea](https://github.com/oryon-dominik/interstellar-gaffa/commit/7ed3bea48a929df57b7374b1b3aaf339eae309cd))
- **(output)** one stream handler reads stdout and stderr — the two tasks were copies apart from three details - ([ef35bb3](https://github.com/oryon-dominik/interstellar-gaffa/commit/ef35bb332c8355c865b745b4f1e99e5f492419c9))

#### Styling

- **(output)** the log line spells its line break — a literal newline split the format string - ([79a53a5](https://github.com/oryon-dominik/interstellar-gaffa/commit/79a53a5164bc35021d437562cfa894507e5c5ccc))



### Package

#### Build

- **(package)** keywords name only what the package does — curated, no search bait - ([beee7bf](https://github.com/oryon-dominik/interstellar-gaffa/commit/beee7bf9807699c6687683c68de1ea100942b519))
- **(package)** unused uuid, a redundant test target, featureless docs.rs flags and ignored badges go - ([72a167a](https://github.com/oryon-dominik/interstellar-gaffa/commit/72a167acefac46f21516c0c2881b88574a0bd8b9))



### Readme

#### Documentation

- **(readme)** badges for crate, tests and licence — the crate page carried none - ([082621c](https://github.com/oryon-dominik/interstellar-gaffa/commit/082621c191fd25bd8f0edeb602482717e678ce01))



### Release

#### Continuous Integration

- **(release)** tests gate every release — a red suite mints no version - ([01b4f73](https://github.com/oryon-dominik/interstellar-gaffa/commit/01b4f738ec59b8a13a0764307395ce70fd1056dd))
- **(release)** comments name a CalVer tag and the time zone — v0.5.1 and an in-house term misled readers - ([d535ac2](https://github.com/oryon-dominik/interstellar-gaffa/commit/d535ac219051efed65469ea54786bedb98229f09))



### Shell

#### Testing

- **(shell)** check the full pwsh path on Windows only — Linux keeps backslashes in the name - ([995bd2a](https://github.com/oryon-dominik/interstellar-gaffa/commit/995bd2a6d2d40566ab6718cbba01a21a98ece603))



### Stability

#### Testing

- **(stability)** let the process outlive the first handle check — echo exited before it - ([073d611](https://github.com/oryon-dominik/interstellar-gaffa/commit/073d611934c4920108639cb3e5536f548500d3c8))



### Status

#### Refactor

- **(status)** one runtime and restart label serves the CLI table and the TUI panel — the same formatting stood in both - ([93bea1d](https://github.com/oryon-dominik/interstellar-gaffa/commit/93bea1daaca2039ae9d692a3bb7d5153a7b19a7a))



### Test

#### Documentation

- **(test)** the shutdown tests' summary names the terminal-controls test too — it covered processes only - ([e93dab7](https://github.com/oryon-dominik/interstellar-gaffa/commit/e93dab7185b51f22d0e2d07d550a96b6121e4e70))



### Ui

#### Bug Fixes

- **(ui)** the input cursor steps over whole characters — typing a non-ASCII character panicked the TUI - ([8420a91](https://github.com/oryon-dominik/interstellar-gaffa/commit/8420a91ed462fd62e2c76acf2d6a2c548109acc3))
- **(ui)** the TUI names the Procfile it loaded — it said Procfile whatever --procfile gave - ([9f8ef77](https://github.com/oryon-dominik/interstellar-gaffa/commit/9f8ef7733cf2d13b52d02f43d2e78ce61e22bfdc))
- **(ui)** run_terminal_ui returns the UI task's error — a failed terminal setup ended as success - ([4916ea7](https://github.com/oryon-dominik/interstellar-gaffa/commit/4916ea739807500c1c3b11fd3d9086870f7a6db8))
- **(ui)** the TUI puts back the panic hook it found — it dropped main's terminal reset - ([575bb19](https://github.com/oryon-dominik/interstellar-gaffa/commit/575bb19a8f37676a6d4e472cad122b656039f9fa))
- **(ui)** the input cursor counts display columns — after CJK or emoji it stood left of the caret - ([80ec8b1](https://github.com/oryon-dominik/interstellar-gaffa/commit/80ec8b1dc8265ad1a3f932352a294ac8067e0032))
- **(ui)** gaffa's own lines are coloured by their mark — a process named gaffa showed as a system message - ([d25bdfd](https://github.com/oryon-dominik/interstellar-gaffa/commit/d25bdfdbf7675fd9c3b5acddece9d8600c457cd9))
- **(ui)** a signal quits the terminal UI like Ctrl+C — its processes outlived gaffa on Unix - ([ee8b33f](https://github.com/oryon-dominik/interstellar-gaffa/commit/ee8b33f2e530352f485e0a7582ebf33a8c77064e))
- **(ui)** the last stop waits for the startup to finish — a process starting late outlived gaffa - ([0432b78](https://github.com/oryon-dominik/interstellar-gaffa/commit/0432b7875335ca888038af07c31b12813c3788a6))
- **(ui)** terminal controls go only to a terminal — a redirected UI wrote escape sequences and stty errors - ([1db0d78](https://github.com/oryon-dominik/interstellar-gaffa/commit/1db0d7868ac5657b624d98177c1b739c22fd0a3c))

#### Refactor

- **(ui)** comments say what the code does now — they named main.rs, a 2 s interval and replaced designs - ([907fb26](https://github.com/oryon-dominik/interstellar-gaffa/commit/907fb264c49b4b2dcd0d3587e1b6d9fb1391a966))

#### Testing

- **(ui)** the wait for the UI task stands on its own and is tested — run_terminal_ui's error return needed a terminal to check - ([309bd6a](https://github.com/oryon-dominik/interstellar-gaffa/commit/309bd6a7c782b3d0ec0f728c7415aa9ac5463b10))



---
## [2026.11.0](https://github.com/oryon-dominik/interstellar-gaffa/compare/2026.9.1..2026.11.0) - 2026-11-05

### Git

#### Chores

- **(git)** ignore agent worktrees — separate checkouts never belong in the index - ([881cfc0](https://github.com/oryon-dominik/interstellar-gaffa/commit/881cfc01d265b520fb9577d9a13fe9cf0d9cd47e))



### Release

#### Continuous Integration

- **(release)** the release commit signs with the producer trailer — inspections skip it - ([a6d8a49](https://github.com/oryon-dominik/interstellar-gaffa/commit/a6d8a49aef6437639fb66e7858ced48e776fa055))



### Shell

#### Bug Fixes

- **(shell)** run cmd lines that start with a quoted program as typed - ([080cc67](https://github.com/oryon-dominik/interstellar-gaffa/commit/080cc67a042e96503d310d5e14ee0fdb21156aae))



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
