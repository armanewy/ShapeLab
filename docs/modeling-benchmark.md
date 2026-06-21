# Modeling Benchmark

This document defines reference assets for later explicit-topology generator work. Wave 0 only establishes contracts; these assets are not implemented yet.

## Reference Asset 1: Industrial Crate

The Industrial Crate should eventually exercise deterministic part hierarchy, repeated details, mirrored components, hard and smooth boundaries, and semantic surface regions.

Expected parts and features:

- rounded main body
- feet
- raised panels
- corner trim
- swept handles
- repeated bolts
- mirrored parts

Useful contract checks:

- panel and trim faces map to generic `Panel` and `Trim` regions
- bevel bands remain distinct from primary surfaces
- repeated bolts preserve per-instance provenance
- mirrored parts preserve stable part and operation IDs
- topology signatures change only when topology-changing parameters change

## Reference Asset 2: Explicit-Topology Desk Lamp

The Explicit-Topology Desk Lamp should eventually exercise lathe and sweep contracts, articulated part structure, sockets, and clean separated meshes.

Expected parts and features:

- lathed base
- swept articulated stem
- cylindrical joints
- lathed or swept shade
- separate clean parts

Useful contract checks:

- sockets align base, stem, joints, and shade without requiring a DCC hierarchy
- lathed and swept parts report deterministic topology signatures
- joint surfaces carry attachment regions
- shade caps, sides, and bevel bands retain semantic region metadata
- provenance distinguishes generated operation families even after mesh combination
