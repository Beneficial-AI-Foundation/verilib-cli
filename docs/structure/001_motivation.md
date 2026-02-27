# Structure for Verification Projects - Motivation

Similar to Blueprint for theorem-proving projects, we have *Structure* for verification projects. Also known as VeriLib Structure or the *molecular structure* of the project.

## Blueprint

A good example is Terrence Tao's Equational Theories project. You can see in the GitHub repo that the structure of the project is broken up into chapters which are saved in the `/blueprint` folder.

https://github.com/teorth/equational_theories/tree/main/blueprint/src/chapter

Here is the Blueprint website or visualizer that was created from the Blueprint files.

https://teorth.github.io/equational_theories/blueprint/

Here is a visualization of the dependency graph for the project. You can see the atomic dependencies, but not the molecular/chapter structure, which makes it hard to sort through all the atoms.

https://teorth.github.io/equational_theories/blueprint/dep_graph_document.html

The following repo is a fork of the original Equational Theories project: [Sample Blueprint repo]. It contains a `.verilib` folder with artifacts which represent changes to the database made by the VeriLib CLI tool.

- `verilib create`
  - `blueprint.json`
  - `config.json`
  - `structure_files.json`
- `verilib atomize`
  - `structure_meta.json`
- `verilib specify`
  - `/certs/specify/veri*.json`
- `verilib verify`
  - `/certs/verify/veri*.json`

## Structure

We want to create a collection of Structure files for formal verification, similar to the Blueprint files for math theorems. The files will contain the molecular and atomic structure of the project. The molecular structure can then be visualized and explored on the VeriLib website.

Blueprint requires manual updating of the statuses of the theorems and proofs, but VeriLib can automate this update through atomization, specification and verification checks.

The following is a branch of the Dalek-Lite project: [Sample Dalek-Lite repo]. It contains a `.verilib` folder with artifacts which represent changes to the database made by the VeriLib CLI tool.

- `verilib create`
  - `tracked_functions.csv`
  - `config.json`
  - `structure_files.json`
  - `/curve25519-dalek/src/*/*.md`
- `verilib atomize`
  - `atoms.json`
  - `structure_meta.json`
- `verilib specify`
  - `specification.json`
  - `/certs/specify/veri*.json`
- `verilib verify`
  - `verification.json`
  - `/certs/verify/veri*.json`
