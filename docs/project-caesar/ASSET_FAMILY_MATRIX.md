# Project Caesar Asset Family Matrix

| Family | Initial Assets | Runtime Keys | Shape Lab Focus |
| --- | --- | --- | --- |
| River crossing | pile, trestle/deck, ramp | `pile`, `deck`, `ramp` | sockets, support surfaces, bridge walkability, deterministic repeated parts |
| Field fortification | palisade, gate, watchtower | `palisade`, `gate`, `tower` | continuation sockets, blocking silhouettes, elevated platforms, construction phases |
| Camp and logistics | road segment, marching camp, decoy worksite | `road`, `marching_camp`, `decoy_worksite` | low-cost road modules, compact camp grammar, intentional incomplete decoys |
| Command pieces | legion, cavalry, engineer, Gallic guard/reserve/scout, depot and supply markers | command-piece keys such as `roman_legion`, `gallic_scout`, `enemy_depot` | symbolic top-down silhouettes, stable footprints, role/team tags |

## Ownership Boundary

`shape-gamekit` owns runtime-neutral contracts:

- logical footprints
- snap anchors
- support surfaces
- walkable surfaces
- traversal links
- simple collision proxies
- construction phases
- readability requirements
- triangle budgets

`shape-caesar-assets` owns Project Caesar authored content:

- Roman and Gallic naming
- River Bend runtime keys
- family-specific template recipes
- Project Caesar dogfooding pack composition

Gameplay balance remains outside Shape Lab. Timber cost, labor, damage, movement bonuses, AI weights, and scenario rules belong to Project Caesar game code.
