# Build and install release version to global cargo directory
install:
    cargo install --path .

# Build release version without installing
build:
    cargo build --release

check:
	cargo check

# Clean build artifacts
clean:
    cargo clean

[group('development')]
lint:
    #!/usr/bin/env bash
    #
    # Default clippy settings (used by `cargo [build, test]` automatically):
    #
    #   cargo clippy
    #
    # If you want stricter clippy settings, start with the suggestion below
    # and consider adding this `lint` target as a dependency to other just
    # targets like `build` and `test`.
    #
    # --all-targets:  check sources and tests
    # --all-features: check non-default crate features
    # -D warnings:    fail the build when encountering warnings
    #
    cargo clippy --verbose --all-targets --all-features -- -D warnings
