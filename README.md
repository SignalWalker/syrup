# Rexa

<p align="center">
  <a href="https://crates.io/crates/rexa"><img src="https://img.shields.io/crates/v/rexa" alt="crates.io"/></a>
  <a href="https://github.com/SignalWalker/rexa/commits/main"><img src="https://img.shields.io/github/commits-since/SignalWalker/rexa/0.1.0" alt="commits since last release"/></a>
  <a href="https://docs.rs/rexa"><img src="https://img.shields.io/docsrs/rexa" alt="docs.rs"/></a>
  <!-- <a href="https://opensource.org/licenses/lgpl-license"><img src="https://img.shields.io/crates/l/rexa" alt="LGPL 3.0 or later"/></a> --> <!-- TODO :: add this back once the crate is published (it displays the wrong license right now) -->
</p>

A library implementing [OCapN](https://github.com/ocapn/ocapn).

## Progress

- [ ] Async runtime agnostic
- [ ] Test with [ocapn-test-suite](https://github.com/ocapn/ocapn-test-suite) (partially working; requires OnionNetlayer to work)
- Syrup:
  - [ ] `#[derive(Serialize, Deserialize)]` (partial; missing enums, some other features)
  - [ ] Design better way of handling enums
- CapTP:
  - [ ] Crossed Hellos mitigation
  - [ ] Figure out ideal way to prevent reader/writer generics from infecting everything else
    - Right now, it's difficult to write code that can use multiple netlayers at once
  - [ ] Should we store locally exported objects as `Arc<dyn Object>`, or should we use a message channel?
  - [ ] Figure out how to deal with promise pipelining
  - [ ] Third-party handoffs
  - Operations:
    - [x] `op:start-session`
    - [x] `op:deliver-only`
    - [x] `op:deliver`
    - [ ] `op:pick`
    - [x] `op:abort`
    - [ ] `op:listen`
    - [ ] `op:gc-export`
    - [ ] `op:gc-answer`
  - Bootstrap:
    - [x] `fetch`
    - [ ] `deposit-gift`
    - [ ] `withdraw-gift`
  - Promises:
    - [x] `fulfill`
    - [x] `break`
- Netlayers:
  - [ ] Onion Netlayer ([arti](https://gitlab.torproject.org/tpo/core/arti)'s onion service support is shaky right now)
  - [ ] Manage multiple transport types using some sort of netlayer manager struct?
- Locators:
  - [ ] Deserialize from URI

## Examples

- [Sneedchat](https://github.com/signalwalker/sneedchat)

## Etymology

- "R" as in "Rust"
- "Exa" as in:
  - "[exo](https://en.wiktionary.org/wiki/exo-)", a prefix meaning "outer" or "external"
  - "[Exapunks](https://www.zachtronics.com/exapunks/)", a puzzle game about distributed programming
