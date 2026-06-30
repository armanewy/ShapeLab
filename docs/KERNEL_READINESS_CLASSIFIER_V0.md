# Kernel Readiness Classifier v0

`KernelReadinessReport` is a safety classifier and the first guardrail for keeping Object Orchard flexible without overclaiming arbitrary family support.

## Problem

Users may eventually ask for many families:

- boxes;
- panels;
- doors;
- stools;
- stoves;
- cars;
- weapons;
- game props.

The product must not pretend that every request can become a usable family immediately.

## Readiness states

### Ready

The request fits a proven starting point.

Examples:

- Box Primitive;
- Flat Panel Primitive.

### Draftable

The request can produce an internal draft, but it is not product-approved.

Examples:

- door-like panel;
- stool-like standing object;
- stove-like appliance.

### Blocked

The request depends on unsupported capabilities or too much complexity.

Examples:

- vehicle families in v0;
- open/close motion before a mechanical motion gate;
- UV/texturing requests before surface work exists;
- rigging/animation requests before the motion stack is active;
- game-ready requests before the full asset-readiness bar exists.

## User copy rule

Readiness reports must use plain language.

They must not expose internal terms such as:

- kernel;
- module;
- provider;
- slot;
- placement zone;
- candidate strategy;
- quality gate;
- topology;
- fingerprint;
- artifact;
- UV.

## Current v0 behavior

The classifier is intentionally small.

It supports only enough logic to protect the current product truth:

- box-like requests can start from Box Primitive;
- panel-like requests can start from Flat Panel Primitive;
- door requests can start as draftable panel work, but cannot claim Door behavior;
- vehicle requests are blocked.

No Door, vehicle, UV/texturing, rigging/animation, or game-ready support is implied by this v0 classifier.

Future kernels should be added one proof at a time.
