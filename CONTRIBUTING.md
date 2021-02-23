First of all, thank you for taking the time to contribute.  This project and everyone participating in it is governed by the [Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

# Reporting Bugs

Bug reports are tracked as [GitHub issues](https://github.com/maldrake/mobc-boltrs/issues). When creating a bug report, please provide as much information as possible. At minimum, provide a description of the steps that you followed that triggered the bug, what you expected to happen, and what actually happened. If you can, provide the source code you wrote that produced that the unexpected result. When providing the source code that triggered the bug, it's helpful to make it as simple and minimal an example as possible while still demonstrating the bug. Removing extra complexity makes diagnosing problems easier.

# Suggesting Enhancements

Feature enhancements are tracked as [Github issues](https://github.com/maldrake/mobc-boltrs/issues). When requesting a feature enhancement, please provide a step-by-step description of the enhancement with as much detail as possible.

# Contributing Code

Thank you for considering a contribution of code to the project.

## Prerequisites

The steps below make the following assumptions.

1. You have Rust installed.
2. You have Docker installed. (Used for testing.)
3. You have cloned the project repository.


## Build

`cargo build`

## Test

Testing requires a database running, to which to connect.

### Environment Variables

Set the following environment variables with parameters for connection to your database.

```
export BOLT_ADDR=127.0.0.1:7687
export BOLT_USER=neo4j
export BOLT_PASS=my-test-password
```

### Run the Database

`docker run --rm -e NEO4J_AUTH="${BOLT_USER}/${BOLT_PASS}" -p 7474:7474 -p 7687:7687 neo4j:4.1`

### Run Tests

`cargo test`

### Lint Code

`cargo clippy --all-targets --all-features -- -D warnings`

### Check Dependencies for Vulnerabilities

`cargo audit`

### Format Code

`cargo fmt`
