use super::api;
use lmdb::Transaction;
use std::path::Path;

pub struct LmdbWrapper {
    env: lmdb::Environment,
    db: lmdb::Database,
}

impl api::Db for LmdbWrapper {
    fn clear(&mut self) -> Result<bool, &'static str> {
        let res = self.env.begin_rw_txn();
        match res {
            Err(_e) => Err("begin-rw-txn failed"),
            Ok(mut txn) => match txn.clear_db(self.db) {
                Err(_e) => Err("clear_db failed"),
                Ok(_) => match txn.commit() {
                    Err(_e) => Err("commit failed"),
                    Ok(_) => Ok(true),
                },
            },
        }
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
        let res = self.env.begin_ro_txn();
        match res {
            Err(_e) => Err("begin-ro-txn failed"),
            Ok(txn) => match txn.get(self.db, &key.to_vec()) {
                Err(e) => {
                    if e == lmdb::Error::NotFound {
                        Ok(None)
                    } else {
                        Err("get failed")
                    }
                }
                Ok(data) => {
                    let v = data.to_vec();
                    txn.abort();
                    Ok(Some(v))
                }
            },
        }
    }

    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<bool, &'static str> {
        let res = self.env.begin_rw_txn();
        match res {
            Err(_e) => Err("begin-rw-txn failed"),
            Ok(mut txn) => match txn.put(
                self.db,
                &key.to_vec(),
                &val.to_vec(),
                lmdb::WriteFlags::empty(),
            ) {
                Err(_e) => Err("put failed"),
                Ok(_) => match txn.commit() {
                    Err(_e) => Err("commit failed"),
                    Ok(_) => Ok(true),
                },
            },
        }
    }

    fn del(&mut self, key: &[u8]) -> Result<bool, &'static str> {
        let res = self.env.begin_rw_txn();
        match res {
            Err(_e) => Err("begin-rw-txn failed"),
            Ok(mut txn) => match txn.del(self.db, &key.to_vec(), None) {
                Err(e) => {
                    if e == lmdb::Error::NotFound {
                        Ok(false)
                    } else {
                        Err("del failed")
                    }
                }
                Ok(_) => match txn.commit() {
                    Err(_e) => Err("commit failed"),
                    Ok(_) => Ok(true),
                },
            },
        }
    }

    fn apply_batch(&mut self, batch: &api::Batch) -> Result<bool, &'static str> {
        let res = self.env.begin_rw_txn();
        match res {
            Err(_e) => Err("begin-rw-txn failed"),
            Ok(mut txn) => {
                for dbm in &batch.ops {
                    match dbm.op {
                        api::MutationOp::Insert => {
                            let value = dbm.value.clone().unwrap();
                            let res = txn.put(self.db, &dbm.key, &value, lmdb::WriteFlags::empty());
                            if res.is_err() {
                                return Err("txn.put failed");
                            }
                        }
                        api::MutationOp::Remove => {
                            let res = txn.del(self.db, &dbm.key, None);
                            if res.is_err() {
                                return Err("txn.put failed");
                            }
                        }
                    }
                }

                match txn.commit() {
                    Err(_e) => Err("commit failed"),
                    Ok(_) => Ok(true),
                }
            }
        }
    }
}

pub struct LmdbDriver {}

impl api::Driver for LmdbDriver {
    fn start_db(&self, cfg: api::Config) -> Result<Box<dyn api::Db + Send>, &'static str> {
        let mut cfg_builder = lmdb::Environment::new();
        if cfg.read_only {
            cfg_builder = *cfg_builder.set_flags(lmdb::EnvironmentFlags::READ_ONLY);
        }
        let path = Path::new(&cfg.path);

        let db_env_res = cfg_builder.open(path);
        match db_env_res {
            Err(_e) => Err("env-open failed"),
            Ok(env) => {
                let db = env.create_db(None, lmdb::DatabaseFlags::empty()).unwrap();
                Ok(Box::new(LmdbWrapper { env, db }) as Box<dyn api::Db + Send>)
            }
        }
    }
}

pub fn new_driver() -> Box<dyn api::Driver> {
    Box::new(LmdbDriver {})
}

#[cfg(test)]
use super::api::{Batch, ConfigBuilder};
#[cfg(test)]
use tempdir::TempDir;

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_get_put() {
        let tmp_dir = TempDir::new("tgp").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(Some(Vec::from("alan"))));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"never_existed"), Ok(None));
    }

    #[test]
    fn test_del() {
        let tmp_dir = TempDir::new("td").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(false));
    }

    #[test]
    fn test_batch() {
        let tmp_dir = TempDir::new("tb").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));

        let mut batch = Batch::default();
        batch.insert(b"age", b"25");
        batch.insert(b"city", b"anytown");
        batch.remove(b"name");
        assert_eq!(db.apply_batch(&batch), Ok(true));

        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"age"), Ok(Some(Vec::from("25"))));
        assert_eq!(db.get(b"city"), Ok(Some(Vec::from("anytown"))));
    }

    #[test]
    fn test_clear() {
        let tmp_dir = TempDir::new("tc").unwrap();
        let tmp_path = tmp_dir.path().to_str().unwrap().to_string();
        let db_config = ConfigBuilder::new().path(tmp_path).read_only(false).build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.put(b"age", b"25"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(Some(Vec::from("alan"))));
        assert_eq!(db.clear(), Ok(true));
        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"age"), Ok(None));
    }
}
