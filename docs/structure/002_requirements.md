# Structure for Verification Projects - Requirements

## Conceptual Overview of VeriLib Structure

- **Structure == Cognitive Structure** (will not call it Blueprint from now on)
- cognitive structure of project defined by topics and stubs
- code hierarchy of project defined by folders/modules and impls/specs/proofs (impl = implementation)
- cognitive structure can be different from code hierarchy
- different kinds of atoms: stubs, impls, specs, proofs
- different kinds of molecules: topics, folders, modules
- different kinds of dependencies: stub deps, impl deps, spec deps, proof deps
- different kinds of viewers: stub viewer, impl viewer, spec viewer, proof viewer, multimodal viewer
- cognitive structure can overlap with code hierarchy
  - some stubs are IDENTIFIED with impls
  - some impls are not stubs (e.g. test functions)
  - some stubs are not impls (e.g. natural language stubs)
- in the absence of structure files
  - default to code hierarchy and generate appropriate structure files
  - generated structure files need not be committed to github
  - but users can choose to commit these structure files (preferably generated locally, not downloaded from verilib)
- besides .md files, there could be other formats for structure files in future, e.g. LaTeX

## Conceptual Overview of VeriLib Web

- we currently have a viewer and editor for code hierarchy
- but we need a viewer for the cognitive structure (a new mode)
- therefore, what we are essentially doing here is generalizing the viewer to work with different modes
- eventually, once we figure out how to sync well with github, we can implement an editor for different modes
- later, we may also have a dependency explorer for different modes where we ignore the molecular structure and can view several levels of dependencies for a given atom all at once (similar to Lara's call graph explorer)

## Conceptual Overview of this MVP

- **MVP == Stub Viewer**
- for this MVP, the aim is to visualize the cognitive structure of the project
- stub dependencies can only be code dependencies (comes from atomization) for now
- arbitrary stub dependencies not allowed for now, but will be needed in future (e.g. "this theorem stub will depend on the following lemma stubs")

## Workflow

### Initialization

- The user creates structure files.
- Files are committed to repo
- The user calls `verilib create` which
  - Asks questions about implementation language, proof language, etc
  - Creates a remote VeriLib repo
  - Creates a local config file
  - Performs structure checks

### Changes

- The user makes changes to the structure files (manually)
- The user makes changes to the source code
- The user (or the CI workflow) calls `verilib atomize` to perform structure checks
- The user calls `verilib specify` to validate and sign the specifications if necessary
- The user calls `verilib verify` to verify and sign the proofs if necessary
- The user commits the changes to the GitHub repo

### VeriLib web

- Automatically checks the repo branch to see if there is a need to reclone
- Performs atomization checks
- Performs specification checks
- Performs verification checks
- In the future, we should be able to view other commits and other branches

## Config File

A `config.json` file in the `.verilib` folder with:
- The VeriLib repo ID
- The VeriLib URL
- The implementation language, e.g. Rust
- The proof language, e.g. Verus, Lean
- The structure format, e.g. Structure, Blueprint
- The structure root path, e.g. `/structure`

## Atom Names

- I prefer calling the unique identifier for functions/definitions/theorems `code-name` rather than `scip-name` because for Lean, we could be using Pantograph for atomization, instead of a SCIP-based atomizer for Lean.
- The `code-name` will start with the scheme, e.g. `scip:...` or `panto:...` to distinguish between the different naming formats

## Structure Files

- The user selects a folder (e.g. the root `/` of the repo, or the `/structure` folder), called the *structure root*, where the molecular structure will be stored.
  - The folders in the structure root will be the molecules.
  - The atoms will be `.md` files nested in the structure root.
- Each `.md` atom file contains an informal spec, code and/or proof of the atom.
- When the user has created source code (e.g. a Rust file) that contains the formal spec, code, and/or proof of the atom and wants to connect it to the atom, they can add its `code-path`, `code-line` as YAML frontmatter in the `.md` atom file. Or they can provide the `scip-name` of the function if known. We can use verilib-cli to autofill the `scip-name` from the code path and line.

Example:
```yaml
---
code-line: 497
code-path: /curve25519-dalek/src/backend/serial/u64/scalar.rs
scip-name: curve25519-dalek/4.1.3/scalar/u64/serial/backend/Scalar52<&Scalar52>#add()
veri-name: scalar/main-theorem
---
```

- Optionally, the user can provide a `veri-name` for the atom (e.g. `scalar/main-theorem`).
- The user can also write down some informal dependencies, which is useful at the start of the project when there is no source code and so the dependencies cannot be computed. The dependencies are given as a YAML list in the frontmatter. Atoms in the dependencies can be identified by either their scip-names, or their veri-names (e.g. `veri:scalar/main-theorem`). Note that molecular paths are not used because they break when the atoms are moved around.
- [???] We will need to figure out a way to store the specification certs and the verification certs. Perhaps in the `.verilib` metadata folder, in a folder where the certs are stored as separate files and named after their scip-name or veri-name.
- The formal dependencies, code-line-endings, code-text can all be extracted from the source code in the repo, and should therefore not be committed to the repo.
- The scip-names should be used as the official identifiers of atoms used throughout the verification project. This is so that we avoid referring to the atoms by their code paths and lines, or by their molecular paths, which are all brittle.

## Structure Checks

- The goal is to check the structure stub files for coherence
- Can be triggered independently with command `verilib atomize`
- Replaces `code-path`, `code-line` in `.md` atom files with `scip-name`
- Check that `veri-name`'s are unique
- Check that dependencies are `scip-name`'s or `veri-name`'s
  - e.g. `scip:curve25519-dalek/.../Scalar52<&Scalar52>#add()`
  - e.g. `veri:scalar/main-theorem`

## Atomization Checks

- The goal is to compute the dependencies and the code content of each atom.
- There are many kinds of atoms
  - Structure stubs (.md files which are placeholders for future functions or theorems)
  - Code functions (implementation is transparent to other atoms)
  - Spec definitions (term is transparent to other atoms)
  - Spec theorems (proof is opaque to other atoms)
- There are many kinds of dependencies
  - Stub dependencies (informal relationships between stubs)
  - Type/spec dependencies (atoms used by the type/spec)
  - Term/proof dependencies (atoms used by the term/proof)
  - Transpilation dependencies (e.g. this Lean function is a transpilation of this Rust function)
- There are many kinds of statuses
  - Type statuses are also called *specification* statuses
  - Term statuses are also called *verification* statuses

## About Overwriting .md Stub Files

- on verilib server, if ALL the .md files are missing, generate .md files from code hierarchy (I can create a script for this)
- locally, user can choose to generate ANY missing .md files using `verilib create` (e.g. generate them from CSV file) and commit them, but this is not necessary since verilib can handle repos without .md files
- user should not be downloading .md files from verilib
- for .md files with only code-path and code-line, user can choose to fill in code-name by running `verilib atomize` locally because the code-name is more stable. After filling in the stub files, the user can commit the changes to the repo. When a .md file has conflicting code-line/code-path and code-name info, the code-name will always take precedence. The user can even choose to delete the code-lines/code-paths from the stub files and commit the changes if they want.
- .md files will not be overwritten during specification/verification, since spec and verification certs are stored separately, as Nima mentioned

## Specification Checks

- The goal is to update the specification statuses of the atoms.
- There are many kinds of specification statuses
  - No spec
  - Only informal spec written
  - Formal spec written
  - Formal spec validated

### Specification certs workflow

1. The user runs `verilib specify` locally prior to making a commit to the repo.
2. The CLI tool first checks if the user has permissions to validate the spec, and if there is a link to the private key of the user.
3. It will then check the existing specification certs (which contain a checksum of the spec previously validated) against the current list of specs (those that contains ensures or requires), and show the user a menu of these new/changed specs.
4. The user selects a spec, and the CLI tool will show the diff in the specs.
5. If the diff looks good, the user chooses accept.
6. The new spec will be signed with the user's private key and then stored in the repo.
7. The user then commits the changes.

### Specification certs storage

Specification certs will be stored on VeriLib and in the GitHub repo:
- A cert is a single file with metadata and cryptographic hashes
  - The scip-name of the function that was specified
  - The hash of the spec of the function*
  - (In future, the hash of spec dependencies*)
  - The name of the person who validated the spec
  - The public key of the person who validated the spec*
  - The timestamp when the spec was validated*
  - The specification hash of the above information with *
- For the MVP, the spec cert will just contain a timestamp.

## Why Spec Certs?

Why do we issue spec validation certs signed by the author, instead of just getting the author from the commit where the spec was added to the repo?

- Because sometimes, committed specs are incomplete (e.g. author is still playing around with certain parameters in the specs)
- Because sometime, committed specs are tentative (e.g. specs are proposed by one person and validated by another)
- Because you can fudge the authors and dates in the commit history with rebases or merges
- Because you may want to transfer the spec and its cert from one repo to another without needing a revalidation by the author (e.g. you can move GitHub Verified Commits from one repo to another as well)

## Verification Checks

- The goal is to update the verification statuses of the atoms.
- There are many kinds of verification statuses
  - No proof
  - Only informal proof written
  - Incomplete formal proof written (with sorry's)
  - Complete formal proof written
  - Complete formal proof verified, but dependences may not be verified
  - Complete formal proof verified, and all dependencies verified.

### Verification certs

- in MVP, all verification statuses MUST come from compilation, not read from a cert
- so for MVP, these compilation results can be stored in the VeriLib DB but should not be committed in the git repo
- in future, we can store cryptographically-signed verification certs in the git repo, to avoid recompiling everything
- the cryptographic signing will prevent users from changing the verification status without calling verilib CLI tool

### Verification certs storage

Verification certs will be stored on VeriLib and in the GitHub repo:
- For MVP, a single cert is generated remotely by the VeriLib server for the whole package. The cert can be committed to the git repo if desired.
- In future, a cert for individual functions is a file with metadata and cryptographic hashes
  - The scip-name of the function that was verified
  - The specification hash of the spec of the function*
  - The hash of the proof of the function*
  - (In future, the hash of proof dependencies*)
  - The name and version of the agent that verified the proof (e.g. VeriLib)
  - The public key of the agent who verified the proof*
  - The name and version of the proof checker
  - The hash of the proof checker code*
  - The timestamp when the proof was verified*
  - The verification hash of the above information with *

### Future `verilib verify` workflow

In future, running `verilib verify` will:
- Get existing certs for current atoms from the VeriLib server
- Make a list of current atoms with proofs but without certs and verify them
- Make a list of current atoms whose certs have expired because of edits to the atom

## Blueprint Files

- We want to provide similar support for existing math-theorem-proving projects with Blueprint files.
- Shaowei has a script that can convert the Blueprint files into Structure files, so we can get limited support this way. It will be a quick way to get buy-in for VeriLib from mathematicians doing FV.
