# ADR 0001: Native Application

## Decision

Shape Lab starts as a native Rust desktop application using `eframe`/`egui`.

## Rationale

The product needs local file access, native memory, worker threads, eventual GPU integration, and deterministic offline behavior. A browser or server would add constraints unrelated to the core hypothesis.
