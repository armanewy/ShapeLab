//! Versioned Foundry kit, provider pack, style pack, and review contracts.
//!
//! Kits are a curated content packaging layer. They summarize exact Foundry
//! catalog content for novice product flows, but they do not bypass the
//! compiler, family/style compatibility checks, or HQ review gates.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

include!("kit/contracts.rs");
include!("kit/visibility_validation.rs");
include!("kit/pack_validation.rs");
include!("kit/tests.rs");
