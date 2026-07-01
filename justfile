target := "thumbv7em-none-eabihf"

# List available commands
default:
    @just --list

# --- check ---

# Type-check a binary without producing an artifact (central | peripheral | all)
check bin="all":
    #!/usr/bin/env sh
    if [ "{{bin}}" = "all" ]; then
        cargo check --target {{target}} --bin central
        cargo check --target {{target}} --bin peripheral
    else
        cargo check --target {{target}} --bin {{bin}}
    fi

# --- lint ---

# Run Clippy on a binary (central | peripheral | all)
lint bin="all":
    #!/usr/bin/env sh
    if [ "{{bin}}" = "all" ]; then
        cargo clippy --target {{target}} --bin central -- -D warnings
        cargo clippy --target {{target}} --bin peripheral -- -D warnings
    else
        cargo clippy --target {{target}} --bin {{bin}} -- -D warnings
    fi

# --- fmt ---

# Format source files; pass bin to check only (central | peripheral | all)
fmt bin="all":
    cargo fmt --all

# Check formatting without modifying (central | peripheral | all)
fmt-check bin="all":
    cargo fmt --all -- --check

# --- build ---

# Build a binary in release mode (central | peripheral | all)
build bin="all":
    #!/usr/bin/env sh
    if [ "{{bin}}" = "all" ]; then
        cargo build --release --target {{target}} --bin central
        cargo build --release --target {{target}} --bin peripheral
    else
        cargo build --release --target {{target}} --bin {{bin}}
    fi

# --- ci ---

# Run all checks: fmt-check + lint + check (no build)
ci:
    just fmt-check
    just lint all
    just check all
