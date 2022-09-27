[![crates.io](https://img.shields.io/crates/v/reference-counted-singleton.svg)](https://crates.io/crates/reference-counted-singleton)
[![docs.rs](https://docs.rs/reference-counted-singleton/badge.svg)](https://docs.rs/reference-counted-singleton)
[![license](https://img.shields.io/github/license/koutheir/reference-counted-singleton?color=black)](https://raw.githubusercontent.com/koutheir/reference-counted-singleton/master/LICENSE.txt)

# Reference-Counted Singleton

[`RefCountedSingleton`] is a reference-counted singleton whose protected data
can be recreated as needed.

The protected data is created when [`RefCountedSingleton::get_or_init`]
is called.
That functions returns an [`RCSRef`] reference to the singleton.

[`RCSRef`] instances can be cloned as needed.
The last [`RCSRef`] reference drops the data.
Calling [`RefCountedSingleton::get_or_init`] again recreates the data.

## Versioning

This project adheres to [Semantic Versioning].
The `CHANGELOG.md` file details notable changes over time.

[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
