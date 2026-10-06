# Code style

Use small types and focused traits to make ownership and invariants clear. Keep runtime host types
in adapters and row traversal statically dispatched. Avoid new abstraction layers without a concrete
second use.

## Rust

- Use module-level imports and document public contracts.
- Explain non-obvious behavior and safety requirements beside the operation they qualify.
- Keep borrowed views tied to retained owners. Never manufacture static lifetimes.
- Preserve exact row identity for uninitialized sink tokens and safe partial abandonment.
- Keep source structures required by recorded Boolean packing and retry code-generation evidence.
- Use the host error family for adapter and row errors. Decoder and infrastructure errors are terminal.
- Do not add inline-always annotations or performance claims without matching evidence.

## Documentation

Use simple, connected prose and concrete API names. Keep overview pages short. Link to the full
contract rather than repeating it. Use expandable sections for complete signatures and inventories.
Do not use em dashes, semicolons, or invented measurements in prose.

## Verification

Verification is opt-in. For an authorized run, choose the narrowest relevant crate or fixture.
Preserve compiler, target, CGU, LTO, allocation, and source-state metadata for performance comparisons.
