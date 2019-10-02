
# Server configuration

A JSON configuration file is required, to specify database.  Command line 
options are also available.

## Table of Contents

* ["zeroconf" mode](#zeroconf-mode)
* [Configuration file](#configuration-file)
   * [Section: databases](#section-databases)
   * [Section: Misc. features](#section-misc-features)
* [Command line help](#command-line-help)

## "zeroconf" mode

kvdbd may be used with a single database by specifying command-line
parameters indicating the database backend to use, and the pathname of
the database to open and serve.

Example of exporting database `db1.kv` using database backend `sled`:
```
$ kvdbd --sled ./db1.kv
```

The database configuration available via the root `/` API will display
a configuration similar to
```
{
   "version" : "0.3.1",
   "databases" : [
      {
         "path" : "db1.kv",
         "name" : "db",
         "read_only" : false,
         "driver" : "sled"
      }
   ],
   "name" : "kvdbd"
}
```

## Configuration file

See `example-cfg-kvdbd.json` for an example configuration file.

### Section: databases

The databases section contains a list of objects, each of which
describes a database to configure and expose via this API service.

Specify a short name, local path and other db attributes to configure
each database.

* **name**:  Short URI-compatible name, exposed via API at database
  name.
* **path**:  Local filesystem path to sled db directory.
* **driver**:  Database driver used to load/store data. Only valid value: "sled"
* **read_only**:  True/false:  Open database in read-only mode?

### Section: Misc. features

* **debug**:  Boolean.  true, to enable additional per-request debug output.

## Command line help

Also, limited options are available at the command line.  Run `--help`
to view available options:

```
$ cargo run --bin kvdbd -- --help
```

