# Structure for Verification Projects - Implementation

A prototype with implementation details can be found at
https://github.com/Beneficial-AI-Foundation/verilib-structure

## Implementation Stages

- **Stage 1**: Only reclone and topological layouts. No web edits.
- **Stage 2**: Viewing different branches and commits in a VeriLib repo.
- **Stage 3**: Layouts can be pulled into local machines to be committed.
- **Stage 4**: VeriLib deployment after atomization on local machine.

## Identifiers

- Identifiers for atoms should be used in atomization, specification and verification.
- The atoms can be functions from the implementation source code, theorems and definitions from the proof source code, or nodes in the informal project structure.
- The identifiers cannot be database IDs because GitHub users would not know them.
- The identifiers cannot be the molecular paths of the atoms, because these will change.
- The identifiers for functions should be computable from the implementation source code.

## Rust vs Python

- Lara's scripts are already in Rust
- Shaowei did his prototypes in Python, because it was faster for him, but he can easily convert them to Rust
- Rust will be easier to install (just download binaries; no need to install python or libraries) and faster in the long run

## S3 vs DB

- I agree with Nima that it is more scalable to store generated artifacts on S3 rather than in the DB.
- These artifacts can be downloaded to the frontend, and we can run client Wasm scripts to query over them for things like fast dependency exploration.
- For the MVP, I can see why Armin would prefer the DB since the web interface is currently designed to work with it.

## Atomization

- Compute atoms dictionary
  - The keys are scip-names
  - The values are code-module, code-path, code-lines (start and end), and dependencies.
- Compute lines dictionary
  - The keys are code-lines intervals (with start and end)
  - The values are scip-names.
  - Can do a (log N) lookup using the intervaltree Python library

## Verification

- Either verify all modules in the repo, or verify a single module
- The output is a list of (code-path, code-line, error-msg) of the failed functions
- Get the scip-name from the code-path and code-line from the code-lines dictionary
- We don't want to recompute the scip-atom dictionary with every verification

## Schema of probe outputs

### Schema of `stubs.json` (output of `probe-verus stubify`)

```json
{
  "curve25519-dalek/src/montgomery.rs/MontgomeryPoint.ct_eq(&MontgomeryPoint).md": {
    "code-line": 111,
    "code-path": "curve25519-dalek/src/montgomery.rs",
    "code-name": "probe:curve25519-dalek/4.1.3/montgomery/MontgomeryPoint#ConstantTimeEq<&MontgomeryPoint>#ct_eq()"
  }
}
```

### Schema of `atoms.json` (output of `probe-verus atomize`)

```json
{
  "probe:curve25519-dalek/4.1.3/field/u64/serial/backend/FieldElement51#ConditionallySelectable<&FieldElement51>#conditional_swap()": {
    "display-name": "conditional_swap",
    "dependencies": [
      "scip:curve25519-dalek/4.1.3/subtle_assumes/u64/serial/backend/choice_is_true()",
      "scip:curve25519-dalek/4.1.3/subtle_assumes/u64/serial/backend/conditional_swap_u64()"
    ],
    "code-module": "field/u64/serial/backend",
    "code-path": "curve25519-dalek/src/backend/serial/u64/field.rs",
    "code-text": {
      "lines-start": 687,
      "lines-end": 732
    }
  }
}
```

### Schema of `specs.json` (output of `probe-verus specify`)

```json
{
  "probe:curve25519-dalek/4.1.3/pow2k_lemmas/field_lemmas/lemmas/c2_val()": {
    "code-path": "src/lemmas/field_lemmas/pow2k_lemmas.rs",
    "spec-text": {
      "lines-start": 687,
      "lines-end": 732
    },
    "specified": true
  }
}
```

### Schema of `proofs.json` (output of `probe-verus verify`)

```json
{
  "probe:curve25519-dalek/4.1.3/pow2k_lemmas/field_lemmas/lemmas/c2_val()": {
    "code-path": "src/lemmas/field_lemmas/pow2k_lemmas.rs",
    "code-line": 456,
    "verified": true,
    "status": "success"
  }
}
```

## Certificates

- Specification and verification certificates of an atom need to be stored separately from the other metadata of the atom or from certs of other atoms, so that they can be copied and transported
- When the latest changes to a repo are pulled onto the VeriLib server, previously-unseen certs need to be verified (hashes recomputed, proofs rechecked) before they are added to the database
