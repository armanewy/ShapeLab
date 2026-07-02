# Object Orchard Docs

## Start here

- [Current Product Status](CURRENT_PRODUCT_STATUS.md) is the product truth.
- [Known Limitations](KNOWN_LIMITATIONS.md) is the unsupported-work boundary.
- [Architecture Status](ARCHITECTURE_STATUS.md) is the current architecture
  phase.
- [Cleanup Plan](CLEANUP_PLAN.md) is the active cleanup sequence.

## Current product truth

- [Primitive Direct-Make Vision](PRIMITIVE_DIRECT_MAKE_VISION.md)
- [Active Variation UI Retirement](ACTIVE_VARIATION_UI_RETIREMENT.md)
- [Box and Flat Panel Direct Property UI](BOX_FLAT_PANEL_DIRECT_PROPERTY_UI.md)
- [Sphere Primitive Direct Controls](SPHERE_PRIMITIVE_DIRECT_CONTROLS.md)
- [Panel with Knob Composition Prototype](PANEL_KNOB_COMPOSITION_PROTOTYPE.md)
- [Orchard Control Grammar](ORCHARD_CONTROL_GRAMMAR.md)

## Architecture contracts

- [Public Contracts](contracts.md)
- [Contract Boundaries](CONTRACT_BOUNDARIES.md)
- [Orchard Core Legacy Boundary](ORCHARD_CORE_LEGACY_BOUNDARY.md)
- [AssetRecipe v8 Semantic Shells](ASSET_RECIPE_V8_SEMANTIC_SHELLS.md)
- [AuthoringOp Log v0](AUTHORING_OP_LOG_V0.md)
- [Relationship Contract v0](RELATIONSHIP_CONTRACT_V0.md)
- [Pattern Contract v0](PATTERN_CONTRACT_V0.md)
- [Primitive Property Schema Contracts](PRIMITIVE_PROPERTY_SCHEMA_CONTRACTS.md)
- [Primitive Composition Contracts v0](PRIMITIVE_COMPOSITION_CONTRACTS_V0.md)
- [Safe Primitive Attachment Policy](SAFE_PRIMITIVE_ATTACHMENT_POLICY.md)
- [ADR 0001: Native Application](adr/0001-native-application.md)
- [ADR 0005: Coordinate Conventions](adr/0005-coordinate-conventions.md)
- [ADR 0006: Part-Aware Asset Recipes](adr/0006-part-aware-asset-recipes.md)
- [ADR 0015: Foundry Document And Control Layer](adr/0015-foundry-document-and-control-layer.md)

## Active subsystems

- [ObjectPlan DSL Contracts](OBJECT_PLAN_DSL_CONTRACTS.md)
- [ObjectPlan Materialization Contracts v1](OBJECT_PLAN_MATERIALIZATION_CONTRACTS_V1.md)
- [ObjectPlan Materializer CLI v1](OBJECT_PLAN_MATERIALIZER_CLI_V1.md)
- [ObjectPlan Offline Runner CLI](OBJECT_PLAN_OFFLINE_RUNNER_CLI.md)
- [ObjectPlan Render Evidence v1](OBJECT_PLAN_RENDER_EVIDENCE_V1.md)
- [ObjectPlan Contact Sheet Evidence](OBJECT_PLAN_CONTACT_SHEET_EVIDENCE.md)
- [ObjectPlan Batch Runner v0](OBJECT_PLAN_BATCH_RUNNER_V0.md)
- [ObjectPlan Batch Review v1](OBJECT_PLAN_BATCH_REVIEW_V1.md)
- [ObjectPlan Review UI Internal Gate](OBJECT_PLAN_REVIEW_UI_INTERNAL_GATE.md)
- [Offline LLM Draft Policy v0](OFFLINE_LLM_DRAFT_POLICY_V0.md)
- [Prototype Pack Brief Contracts v0](PROTOTYPE_PACK_BRIEF_CONTRACTS_V0.md)
- [Prototype Pack Brief to ObjectPlan Batch v0](PROTOTYPE_PACK_BRIEF_TO_OBJECT_PLAN_BATCH_V0.md)
- [Family Studio Direct Kit Readiness Gate](FAMILY_STUDIO_DIRECT_KIT_READINESS_GATE.md)
- [Family Studio Lite Direct Kit UI v0](FAMILY_STUDIO_LITE_DIRECT_KIT_UI_V0.md)
- [Direct Kit Contracts v0](DIRECT_KIT_CONTRACTS_V0.md)
- [Direct Kit Test Runner CLI v0](DIRECT_KIT_TEST_RUNNER_CLI_V0.md)
- [Personal Kit Storage v0](PERSONAL_KIT_STORAGE_IMPLEMENTATION_V0.md)
- [Personal Kit UI Contracts v0](PERSONAL_KIT_UI_CONTRACTS_V0.md)

## Export and proof reports

- [ObjectPlan v0 Integration Report](OBJECT_PLAN_V0_INTEGRATION_REPORT.md)
- [ObjectPlan Truth Render Blocker Gate](OBJECT_PLAN_V0_TRUTH_RENDER_BLOCKER_GATE.md)
- [ObjectPlan Materialization v1 Integration Report](OBJECT_PLAN_MATERIALIZATION_V1_INTEGRATION_REPORT.md)
- [Geometry Export Truth Gate](GEOMETRY_EXPORT_TRUTH_GATE.md)
- [Geometry Only Export Contracts v0](GEOMETRY_ONLY_EXPORT_CONTRACTS_V0.md)
- [ObjectPlan Geometry Export CLI v0](OBJECT_PLAN_GEOMETRY_EXPORT_CLI_V0.md)
- [Geometry Export v0 Integration Report](GEOMETRY_EXPORT_V0_INTEGRATION_REPORT.md)
- [Godot Geometry Import Harness v0](GODOT_GEOMETRY_IMPORT_HARNESS_V0.md)
- [Export Report Includes Contract](EXPORT_REPORT_INCLUDES_CONTRACT.md)
- [Export Realization Report v0](EXPORT_REALIZATION_REPORT_V0.md)
- [Phase A-D Semantic Compiler Integration Report](PHASE_A_D_SEMANTIC_COMPILER_INTEGRATION_REPORT.md)
- [Pattern Evaluation Proof v0](PATTERN_EVALUATION_PROOF_V0.md)
- [Direct Make AuthoringOp Bridge v0](DIRECT_MAKE_AUTHORING_OP_BRIDGE_V0.md)
- [Panel Knob Relationship Migration v0](PANEL_KNOB_RELATIONSHIP_MIGRATION_V0.md)
- [Direct Kit Family Studio Integration Report](DIRECT_KIT_FAMILY_STUDIO_INTEGRATION_REPORT.md)

## Internal gates

- [Test Gate Policy](TEST_GATE_POLICY.md)
- [Product Claim Gate](PRODUCT_CLAIM_GATE.md)
- [Codebase Hygiene Policy](CODEBASE_HYGIENE_POLICY.md)
- [Evidence Manifest Policy](EVIDENCE_MANIFEST_POLICY.md)
- [Rust File Size Exceptions](RUST_FILE_SIZE_EXCEPTIONS.md)
- [Development Speed](DEVELOPMENT_SPEED.md)
- [Building](building.md)
- [Semantic Clay Preview Mode](SEMANTIC_CLAY_PREVIEW_MODE.md)

## Deleted/obsolete documentation policy

Docs from retired pivots must be deleted instead of carried as parallel
status. This includes Sci-Fi Crate, Cargo Case, crate-family, generated
variation, candidate-tray, old dogfood, old showcase, old hero-template, old
DCC, and old game-ready package reports unless a current test or contract
explicitly revalidates the file.

The active docs may mention retired names only to say they are not the active
direction. They must not describe generated variation UI, random candidate
generation, material/UV workflows, collision, rigging, animation, Godot-ready
output, or game-ready output as supported product capability.
