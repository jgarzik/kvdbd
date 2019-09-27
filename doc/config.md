
# Server configuration

A JSON configuration file is required, to specify database.  Command line 
options are also available.

## Table of Contents

* [Configuration file](#configuration-file)
   * [Section: databases](#section-databases)
   * [Section: Misc. features](#section-misc-features)
* [Command line help](#command-line-help)

## Configuration file

See `example-cfg-kvdb.json` for an example configuration file.

### Section: databases

The databases section contains a list of objects, each of which
describes a database to configure and expose via this API service.

Specify a short name, local path and other db attributes to configure
each database.

* **name**:  Short URI-compatible name, exposed via API at database
  name.
* **path**:  Local filesystem path to sled db directory.
* **driver**:  Database driver used to load/store data. Only valid value: "sled"

### Section: Misc. features

* **debug**:  Boolean.  true, to enable additional per-request debug output.

## Command line help

Also, limited options are available at the command line.  Run `--help`
to view available options:

```
$ cargo run --bin kvdb -- --help
```

