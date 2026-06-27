# Game Asset Layer Lineage

Game asset layer references describe how generated artifacts bind to frozen mesh
and topology state.

Layers:

- mesh
- surface
- rig
- motion
- collision
- export

`GameAssetLayerRef` records the layer kind, artifact reference, frozen mesh
fingerprint, topology fingerprint, and optional parent artifact. Surface binds
to frozen mesh. Rig binds to frozen mesh and topology. Motion binds to a rig
artifact. Frozen topology changes invalidate surface, rig, motion, and export
layers because rig and clip evidence were authored against a specific topology.
Material-only variants preserve rig and motion lineage when mesh and topology
fingerprints are unchanged.
