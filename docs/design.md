# Structure for Verification Projects

**Author:** Shaowei Lin

## 1. Introduction

### 1.1 Blueprint

Similar to Blueprint for theorem-proving projects, we have Structure for verification
projects. Also known as VeriLib Structure or the molecular structure of the project.

A good example is Terrence Tao's Equational Theories project. You can see in the
[GitHub repo](https://github.com/teorth/equational_theories/tree/main/blueprint/src/chapter)
that the structure of the project is broken up into chapters which are saved in the
`/blueprint` folder.

Here is the [Blueprint website](https://teorth.github.io/equational_theories/blueprint/)
or visualizer that was created from the Blueprint files.

Here is a [visualization of the dependency graph](https://teorth.github.io/equational_theories/blueprint/dep_graph_document.html)
for the project. You can see the atomic dependencies, but not the molecular/chapter
structure, which makes it hard to sort through all the atoms.

The following repo is a fork of the original Equational Theories project. It contains a
`.verilib` folder with artifacts which represent changes to the database made by the
VeriLib CLI tool:

- `verilib create` — `blueprint.json`, `config.json`, `structure_files.json`
- `verilib atomize` — `structure_meta.json`
- `verilib specify` — `/certs/specify/veri*.json`
- `verilib verify` — `/certs/verify/veri*.json`

### 1.2 Structure

We want to create a collection of Structure files for formal verification, similar to
the Blueprint files for math theorems. The files will contain the molecular and atomic
structure of the project. The molecular structure can then be visualized and explored on
the VeriLib website.

Blueprint requires manual updating of the statuses of the theorems and proofs, but VeriLib
can automate this update through atomization, specification and verification checks.

The following is a branch of the Dalek-Lite project. It contains a `.verilib` folder with
artifacts which represent changes to the database made by the VeriLib CLI tool:

- `verilib create` — `tracked_functions.csv`, `config.json`, `structure_files.json`, `/curve25519-dalek/src/*/*.md`
- `verilib atomize` — `atoms.json`, `structure_meta.json`
- `verilib specify` — `specification.json`, `/certs/specify/veri*.json`
- `verilib verify` — `verification.json`, `/certs/verify/veri*.json`

## 2. Requirements

### 2.1 Conceptual Overview of VeriLib Structure

- Structure == Cognitive Structure (will not call it Blueprint from now on)
- Cognitive structure of project defined by topics and stubs
- Code hierarchy of project defined by folders/modules and impls/specs/proofs (impl = implementation)
- Cognitive structure can be different from code hierarchy
- Different kinds of atoms: stubs, impls, specs, proofs
- Different kinds of molecules: topics, folders, modules
- Different kinds of dependencies: stub deps, impl deps, spec deps, proof deps
- Different kinds of viewers: stub viewer, impl viewer, spec viewer, proof viewer, multimodal viewer
- Cognitive structure can overlap with code hierarchy
  - Some stubs are IDENTIFIED with impls
  - Some impls are not stubs (e.g. test functions)
  - Some stubs are not impls (e.g. natural language stubs)
- In the absence of structure files
  - Default to code hierarchy and generate appropriate structure files
  - Generated structure files need not be committed to GitHub
  - But users can choose to commit these structure files (preferably generated locally, not downloaded from VeriLib)
- Besides `.md` files, there could be other formats for structure files in future, e.g. LaTeX

### 2.2 MVP Overview

- MVP == Stub Viewer
- For this MVP, the aim is to visualize the cognitive structure of the project
- Stub dependencies can only be code dependencies (comes from atomization) for now
- Arbitrary stub dependencies not allowed for now, but will be needed in future (e.g. "this theorem stub will depend on the following lemma stubs")

### 2.3 Workflow

- **Initialization**
  - The user creates structure files.
  - Files are committed to repo.
  - The user calls `verilib create` which:
    - Asks questions about implementation language, proof language, etc.
    - Creates a remote VeriLib repo.
    - Creates a local config file.
    - Performs structure checks.
- **Changes**
  - The user makes changes to the structure files (manually).
  - The user makes changes to the source code.
  - The user (or the CI workflow) calls `verilib atomize` to perform structure checks.
  - The user calls `verilib specify` to validate and sign the specifications if necessary.
  - The user calls `verilib verify` to verify and sign the proofs if necessary.
  - The user commits the changes to the GitHub repo.
- **VeriLib web**
  - Automatically checks the repo branch to see if there is a need to reclone.
  - Performs atomization checks.
  - Performs specification checks.
  - Performs verification checks.
  - In the future, we should be able to view other commits and other branches.

### 2.4 Config File

A `config.json` file in the `.verilib` folder with:

- The VeriLib repo ID
- The VeriLib URL
- The implementation language, e.g. Rust
- The proof language, e.g. Verus, Lean
- The structure format, e.g. Structure, Blueprint
- The structure root path, e.g. `/structure`

### 2.5 Atom Names

- The unique identifier for functions/definitions/theorems is called `code-name` rather
  than `scip-name` because for Lean, we could be using Pantograph for atomization,
  instead of a SCIP-based atomizer.
- The `code-name` will start with the scheme, e.g. `scip:...` or `panto:...` to
  distinguish between the different naming formats.

### 2.6 Structure Files

- The user selects a folder (e.g. the root `/` of the repo, or the `/structure` folder),
  called the **structure root**, where the molecular structure will be stored.
  - The folders in the structure root will be the molecules.
  - The atoms will be `.md` files nested in the structure root.
- Each `.md` atom file contains an informal spec, code and/or proof of the atom.
- When the user has created source code that contains the formal spec, code, and/or
  proof of the atom and wants to connect it to the atom, they can add its `code-path`,
  `code-line` as YAML frontmatter in the `.md` atom file. Or they can provide the
  `code-name` of the function if known. We can use verilib-cli to autofill the
  `code-name` from the code path and line.

  Example frontmatter:

  ```yaml
  ---
  code-line: 497
  code-path: /curve25519-dalek/src/backend/serial/u64/scalar.rs
  code-name: curve25519-dalek/4.1.3/scalar/u64/serial/backend/Scalar52<&Scalar52>#add()
  veri-name: scalar/main-theorem
  ---
  ```

- Optionally, the user can provide a `veri-name` for the atom.
- The user can also write down some informal dependencies as a YAML list in the
  frontmatter. Atoms in the dependencies can be identified by either their `code-name`s
  or their `veri-name`s (e.g. `veri:scalar/main-theorem`). Molecular paths are not used
  because they break when atoms are moved around.
- Specification certs and verification certs are stored in the `.verilib` metadata folder,
  in a folder where the certs are stored as separate files named after their `code-name`
  or `veri-name`.
- The formal dependencies, code-line-endings, code-text can all be extracted from the
  source code in the repo, and should therefore not be committed to the repo.
- The `code-name`s should be used as the official identifiers of atoms used throughout
  the verification project, to avoid referring to atoms by their code paths and lines or
  molecular paths, which are all brittle.

### 2.7 Structure Checks

- The goal is to check the structure stub files for coherence.
- Can be triggered independently with command `verilib atomize`.
- Replaces `code-path`, `code-line` in `.md` atom files with `code-name`.
- Check that `veri-name`s are unique.
- Check that dependencies are `code-name`s or `veri-name`s.

### 2.8 Atomization Checks

- The goal is to compute the dependencies and the code content of each atom.
- There are many kinds of atoms:
  - Structure stubs (`.md` files which are placeholders for future functions or theorems)
  - Code functions (implementation is transparent to other atoms)
  - Spec definitions (term is transparent to other atoms)
  - Spec theorems (proof is opaque to other atoms)
- There are many kinds of dependencies:
  - Stub dependencies (informal relationships between stubs)
  - Type/spec dependencies (atoms used by the type/spec)
  - Term/proof dependencies (atoms used by the term/proof)
  - Transpilation dependencies (e.g. this Lean function is a transpilation of this Rust function)
- There are many kinds of statuses:
  - Type statuses are also called specification statuses
  - Term statuses are also called verification statuses

### 2.9 About Overwriting .md Stub Files

- On verilib server, if ALL the `.md` files are missing, generate `.md` files from code
  hierarchy.
- Locally, user can choose to generate ANY missing `.md` files using `verilib create`
  and commit them, but this is not necessary since VeriLib can handle repos without
  `.md` files.
- User should not be downloading `.md` files from VeriLib.
- For `.md` files with only `code-path` and `code-line`, user can choose to fill in
  `code-name` by running `verilib atomize` locally because the `code-name` is more
  stable. After filling in the stub files, the user can commit the changes to the repo.
  **When a `.md` file has conflicting `code-line`/`code-path` and `code-name` info, the
  `code-name` will always take precedence.** The user can even choose to delete the
  `code-line`s/`code-path`s from the stub files and commit the changes if they want.
- **`.md` files will not be overwritten during specification/verification**, since spec
  and verification certs are stored separately.

### 2.10 Specification Checks

- The goal is to update the specification statuses of the atoms.
- There are many kinds of specification statuses:
  - No spec
  - Only informal spec written
  - Formal spec written
  - Formal spec validated
- **Specification certs workflow:**
  - The user runs `verilib specify` locally prior to making a commit to the repo.
  - The CLI tool first checks if the user has permissions to validate the spec, and if
    there is a link to the private key of the user.
  - It will then check the existing specification certs (which contain a checksum of the
    spec previously validated) against the current list of specs (those that contain
    `ensures` or `requires`), and show the user a menu of these new/changed specs.
  - The user selects a spec, and the CLI tool will show the diff in the specs.
  - If the diff looks good, the user chooses accept.
  - The new spec will be signed with the user's private key and then stored in the repo.
  - The user then commits the changes.
- Specification certs will be stored on VeriLib and in the GitHub repo.
  - A cert is a single file with metadata and cryptographic hashes:
    - The `code-name` of the function that was specified
    - The hash of the spec of the function
    - (In future, the hash of spec dependencies)
    - The name of the person who validated the spec
    - The public key of the person who validated the spec
    - The timestamp when the spec was validated
    - The specification hash of the above information
  - **For the MVP, the spec cert will just contain a timestamp.**

### 2.11 Verification Checks

- The goal is to update the verification statuses of the atoms.
- There are many kinds of verification statuses:
  - No proof
  - Only informal proof written
  - Incomplete formal proof written (with sorry's)
  - Complete formal proof written
  - Complete formal proof verified, but dependencies may not be verified
  - Complete formal proof verified, and all dependencies verified
- **Verification certs:**
  - In MVP, all verification statuses MUST come from compilation, not read from a cert.
  - So for MVP, these compilation results can be stored in the VeriLib DB but should not
    be committed in the git repo.
  - In future, we can store cryptographically-signed verification certs in the git repo, to
    avoid recompiling everything.
  - The cryptographic signing will prevent users from changing the verification status
    without calling verilib CLI tool.
- Verification certs will be stored on VeriLib and in the GitHub repo.
  - For MVP, a single cert is generated remotely by the VeriLib server for the whole
    package. The cert can be committed to the git repo if desired.
  - In future, a cert for individual functions is a file with metadata and cryptographic
    hashes:
    - The `code-name` of the function that was verified
    - The specification hash of the spec of the function
    - The hash of the proof of the function
    - (In future, the hash of proof dependencies)
    - The name and version of the agent that verified the proof (e.g. VeriLib)
    - The public key of the agent who verified the proof
    - The name and version of the proof checker
    - The hash of the proof checker code
    - The timestamp when the proof was verified
    - The verification hash of the above information
  - In future, running `verilib verify` will:
    - Get existing certs for current atoms from the VeriLib server
    - Make a list of current atoms with proofs but without certs and verify them
    - Make a list of current atoms whose certs have expired because of edits to the atom

### 2.12 Blueprint Files

- We want to provide similar support for existing math-theorem-proving projects with
  Blueprint files.
- Shaowei has a script that can convert the Blueprint files into Structure files, so we can
  get limited support this way. It will be a quick way to get buy-in for VeriLib from
  mathematicians doing FV.

## 3. Implementation

### 3.1 Implementation Stages

- Stage 1: Only reclone and topological layouts. No web edits.
- Stage 2: Viewing different branches and commits in a VeriLib repo.
- Stage 3: Layouts can be pulled into local machines to be committed.
- Stage 4: VeriLib deployment after atomization on local machine.

### 3.2 Identifiers

- Identifiers for atoms should be used in atomization, specification and verification.
- The atoms can be functions from the implementation source code, theorems and
  definitions from the proof source code, or nodes in the informal project structure.
- The identifiers cannot be database IDs because GitHub users would not know them.
- The identifiers cannot be the molecular paths of the atoms, because these will change.
- The identifiers for functions should be computable from the implementation source code.

### 3.3 Atomization

- Compute **atoms dictionary**:
  - The keys are `code-name`s.
  - The values are `code-module`, `code-path`, `code-lines` (start and end), and `dependencies`.
- Compute **lines dictionary**:
  - The keys are code-lines intervals (with start and end).
  - The values are `code-name`s.
  - Can do a O(log N) lookup using an interval tree.

### 3.4 Verification

- Either verify all modules in the repo, or verify a single module.
- The output is a list of `(code-path, code-line, error-msg)` of the failed functions.
- Get the `code-name` from the `code-path` and `code-line` from the code-lines dictionary.
- We don't want to recompute the atoms dictionary with every verification.

### 3.5 Schema of probe outputs

**stubs.json** (output of `probe-verus stubify`):

```json
{
  "curve25519-dalek/src/montgomery.rs/MontgomeryPoint.ct_eq(&MontgomeryPoint).md": {
    "code-line": 111,
    "code-path": "curve25519-dalek/src/montgomery.rs",
    "code-name": "probe:curve25519-dalek/4.1.3/montgomery/MontgomeryPoint#ConstantTimeEq<&MontgomeryPoint>#ct_eq()"
  }
}
```

**atoms.json** (output of `probe-verus atomize`):

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

**specs.json** (output of `probe-verus specify`):

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

**proofs.json** (output of `probe-verus verify`):

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

### 3.6 Certificates

- Specification and verification certificates of an atom need to be stored separately from
  the other metadata of the atom or from certs of other atoms, so that they can be copied
  and transported.
- When the latest changes to a repo are pulled onto the VeriLib server, previously-unseen
  certs need to be verified (hashes recomputed, proofs rechecked) before they are added
  to the database.
