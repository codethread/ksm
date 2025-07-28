# Build and install release version to global cargo directory
install:
    cargo install --path .

# Build release version without installing
build:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean