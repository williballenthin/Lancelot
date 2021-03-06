# lancelot-flirt

A Rust library for parsing, compiling, and matching [Fast Library Identification and Recognition Technology (FLIRT)](https://hex-rays.com/products/ida/tech/flirt/in_depth/) signatures. These signatures are typically used by the Hex-Rays IDA Pro tool; this library is the result of reverse engineering the matching engine and reimplementing parsers and matchers. You can use this library to match FLIRT signatures against byte sequences to recognize statically-linked code without IDA Pro.

[Python bindings](https://github.com/williballenthin/lancelot/tree/master/pyflirt) generated via
[PyO3](https://github.com/PyO3/pyo3) for Python 3.x are available on PyPI as
[python-flirt](https://pypi.org/project/python-flirt/).

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
lancelot-flirt = "0.6"
```

Here's a sample example that parses a FLIRT signature from a string and matches against a byte sequence:

```rust
use lancelot_flirt;

// signature in .pat file format.
// note: .sig file format also supported, see `lancelot_flirt::sig::*`.
const PAT: &'static str = "\
518B4C240C895C240C8D5C240C508D442408F7D923C18D60F88B43F08904248B 21 B4FE 006E :0000 __EH_prolog3_GS_align ^0041 ___security_cookie ........33C5508941FC8B4DF0895DF08B4304894504FF75F464A1000000008945F48D45F464A300000000F2C3
518B4C240C895C240C8D5C240C508D442408F7D923C18D60F88B43F08904248B 1F E4CF 0063 :0000 __EH_prolog3_align ^003F ___security_cookie ........33C5508B4304894504FF75F464A1000000008945F48D45F464A300000000F2C3
518B4C240C895C240C8D5C240C508D442408F7D923C18D60F88B43F08904248B 22 E4CE 006F :0000 __EH_prolog3_catch_GS_align ^0042 ___security_cookie ........33C5508941FC8B4DF08965F08B4304894504FF75F464A1000000008945F48D45F464A300000000F2C3
518B4C240C895C240C8D5C240C508D442408F7D923C18D60F88B43F08904248B 20 6562 0067 :0000 __EH_prolog3_catch_align ^0040 ___security_cookie ........33C5508965F08B4304894504FF75F464A1000000008945F48D45F464A300000000F2C3
---";

// utcutil.dll
//  MD5 abc9ea116498feb8f1de45f60d595af6
//  SHA-1 2f1ba350237b74c454caf816b7410490f5994c59
//  SHA-256 7607897638e9dae406f0840dbae68e879c3bb2f08da350c6734e4e2ef8d61ac2
// __EH_prolog3_catch_align
const BUF: [u8; 103] = [
    0x51, 0x8b, 0x4c, 0x24, 0x0c, 0x89, 0x5c, 0x24,
    0x0c, 0x8d, 0x5c, 0x24, 0x0c, 0x50, 0x8d, 0x44,
    0x24, 0x08, 0xf7, 0xd9, 0x23, 0xc1, 0x8d, 0x60,
    0xf8, 0x8b, 0x43, 0xf0, 0x89, 0x04, 0x24, 0x8b,
    0x43, 0xf8, 0x50, 0x8b, 0x43, 0xfc, 0x8b, 0x4b,
    0xf4, 0x89, 0x6c, 0x24, 0x0c, 0x8d, 0x6c, 0x24,
    0x0c, 0xc7, 0x44, 0x24, 0x08, 0xff, 0xff, 0xff,
    0xff, 0x51, 0x53, 0x2b, 0xe0, 0x56, 0x57, 0xa1,
    0x70, 0x14, 0x01, 0x10, 0x33, 0xc5, 0x50, 0x89,
    0x65, 0xf0, 0x8b, 0x43, 0x04, 0x89, 0x45, 0x04,
    0xff, 0x75, 0xf4, 0x64, 0xa1, 0x00, 0x00, 0x00,
    0x00, 0x89, 0x45, 0xf4, 0x8d, 0x45, 0xf4, 0x64,
    0xa3, 0x00, 0x00, 0x00, 0x00, 0xf2, 0xc3,
];

// parse signature file content into a list of signatures.
let sigs = lancelot_flirt::pat::parse(PAT).expect("failed to parse signatures");

// compile signatures into a matching engine instance.
// separate from above so that you can load multiple files.
let matcher = lancelot_flirt::FlirtSignatureSet::with_signatures(sigs);

// match the signatures against the given buffer, starting at offset 0.
// results in a list of matches (that are just signature instances).
for m in matcher.r#match(&BUF).iter() {
    println!("match: {}", m.get_name().expect("failed  to extract name"));
}
```

expected output:

```
match: __EH_prolog3_catch_align
```

Note, the above logic does not handle "references" that are describe below;
however, it does give a sense for the required setup to parse and compile rules.

### Usage: signature file formats

This library supports loading signatures from both the .sig and .pat file formats:

  - .sig files are the compiled signatures usually fed into IDA Pro for matching. They are structurally compressed (and uncommonly compressed with a zlib-like algorithm, not supported here) and have a raw binary representation.

  - .pat files are the ASCII-encoded text files generated by `sigmake.exe`. These are typically compiled into .sig files for use in IDA Pro; however, since `lancelot-flirt` compiles the rules into its own intermediate representation, you can use them directly. Notably, this library supports a slight extension to enable a file header with lines prefixed with `#`, which enables you to embed a acknowledgement/copyright/license.

With knowledge of the above, you may consider also supporting `.pat.gz` signature files in your client application, as this enables a great compression ratio while preserving the file license header and human-inspectability.

### Usage: matching references

To differentiate functions with a shared byte-wise representation, such as wrapper functions that dispatch other addresses, a FLIRT engine matches recursively using "references".
This feature is used heavily to match common routines provided by modern C/C++ runtime libraries.

Unfortunately, client code must coordinate the recursive invocation of FLIRT matching.

Therefore, when integrating this library into a client application, you should review the matching logic of `lancelot::core::analysis::flirt` [here](https://github.com/williballenthin/lancelot/blob/master/core/src/analysis/flirt.rs).
Essentially, you'll need to inspect the "references" found within a function and recursively FLIRT match those routines to resolve the best matching signature.
There's also a matching implementation in Python for vivisect [here](https://github.com/williballenthin/viv-utils/blob/master/viv_utils/flirt.py) that relies on more thorough code flow recovery.


### Usage: example tool

The tool `match_flirt` from `lancelot::bin::match_flirt` [here](https://github.com/williballenthin/lancelot/blob/master/bin/src/bin/match_flirt.rs) uses `lancelot-flirt` to recognize statically-linked functions within PE files.
You can use this code as a template for integration this library with your client code.

## License

This project is licensed under the Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0).
You should not redistribute FLIRT signatures distributed by Hex-Rays; however, there are open source signatures available here:

  - https://github.com/fireeye/siglib/
  - https://github.com/Maktm/FLIRTDB
