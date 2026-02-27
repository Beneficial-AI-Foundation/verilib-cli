# Web, CLI, Probe - Motivation

## Relationship between Web, CLI, Probe

* Users can interact with VeriLib through two interfaces  - Web and CLI

* The probe scripts contains functionality that is needed by CLI and Web to support user requests. So hopefully, Web can be decoupled from CLI eventually. 

* The probe scripts are essentially parsers and analyzers. They don't change the source code. They generate output files that contain the parsing and analysis results, which are then used by CLI or Web. They do not filter or delete results unless it is clear that those results will not be used in any situation. 

* The current outputs are: stubs.json, atoms.json, specs.json, proofs.json. We also have spec certificates in .verilib/specs. We can add information to the outputs over time, but hopefully in a way that does not break existing schema. We should make the schema version explicit in the JSON outputs in anticipation of these breaking changes.

## Probe

* Contains installers for tool dependencies, such as SCIP, Verus-Analyzer and Verus
* Subcommands for a variety of applications
    * Projects like VeriLib-CLI may use subcommands like `probe-verus atomize` to get atoms in repo and output to JSON
    * Projects like Dalek-Verus may use special subcommands like `probe-verus tracked_csv` to get additional atom properties and output to CSV
    * Projects like Dalek-Lean may use special subcommands like `probe-lean tracked_csv` to get additional atom properties and output to CSV

## CLI

* Only installs itself. Additional steps needed to install `probe-verus` or `probe-lean` and their dependencies. Checks for these dependencies before calling them within the CLI scripts, and give installation instructions if they are missing.


## Stubs

* In the beginning, there are only .md or .tex stub files. No need to install Lean or Verus till later.

* when calling `probe-XXX stubify`, it can produce `stubs.json` from a variety of sources, like the .md files, latex files, rust files, lean files.

* When do we call `verilib-cli create` to create the first stubs from the existing atoms?

* When do we need to graduate a stub to an actual Lean atom or Rust atom? Think about spec files in Claude Code.


