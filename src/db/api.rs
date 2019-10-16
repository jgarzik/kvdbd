pub enum MutationOp {
    Insert,
    Remove,
}

pub struct Mutation {
    pub op: MutationOp,
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
}

pub struct Batch {
    pub ops: Vec<Mutation>,
}

impl Batch {
    pub fn default() -> Batch {
        Batch { ops: Vec::new() }
    }

    pub fn insert(&mut self, key_in: &[u8], value_in: &[u8]) {
        self.ops.push(Mutation {
            op: MutationOp::Insert,
            key: key_in.to_vec(),
            value: Some(value_in.to_vec()),
        });
    }

    pub fn remove(&mut self, key_in: &[u8]) {
        self.ops.push(Mutation {
            op: MutationOp::Remove,
            key: key_in.to_vec(),
            value: None,
        });
    }
}

pub struct Config {
    pub path: String,
    pub read_only: bool,
}

pub struct ConfigBuilder {
    pub path: Option<String>,
    pub read_only: Option<bool>,
}

pub trait Db {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str>;
    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<bool, &'static str>;
    fn del(&mut self, key: &[u8]) -> Result<bool, &'static str>;
    fn apply_batch(&mut self, batch: &Batch) -> Result<bool, &'static str>;
}

pub trait Driver {
    fn start_db(&self, cfg: Config) -> Result<Box<dyn Db + Send>, &'static str>;
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        ConfigBuilder {
            path: None,
            read_only: None,
        }
    }

    pub fn path(&mut self, path_in: String) -> &mut ConfigBuilder {
        self.path = Some(path_in);
        self
    }

    pub fn read_only(&mut self, val_in: bool) -> &mut ConfigBuilder {
        self.read_only = Some(val_in);
        self
    }

    pub fn build(&self) -> Config {
        Config {
            path: match &self.path {
                None => String::from("./db"),
                Some(p) => String::from(p),
            },
            read_only: match &self.read_only {
                None => false,
                Some(v) => *v,
            },
        }
    }
}

#[cfg(test)]
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    pub struct MemDb {
        db: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl Db for MemDb {
        fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
            match self.db.get(key) {
                None => Ok(None),
                Some(val) => Ok(Some(val.to_vec())),
            }
        }

        fn put(&mut self, key: &[u8], val: &[u8]) -> Result<bool, &'static str> {
            self.db.insert(key.to_vec(), val.to_vec());
            Ok(true)
        }

        fn del(&mut self, key: &[u8]) -> Result<bool, &'static str> {
            match self.db.remove(key) {
                None => Ok(false),
                Some(_v) => Ok(true),
            }
        }

        fn apply_batch(&mut self, batch: &Batch) -> Result<bool, &'static str> {
            for dbm in &batch.ops {
                match dbm.op {
                    MutationOp::Insert => {
                        let val: Vec<u8> = dbm.value.clone().unwrap();
                        self.db.insert(dbm.key.to_vec(), val);
                    }
                    MutationOp::Remove => {
                        self.db.remove(&dbm.key);
                    }
                }
            }

            Ok(true)
        }
    }

    pub struct MemDriver {}

    impl Driver for MemDriver {
        fn start_db(&self, _cfg: Config) -> Result<Box<dyn Db + Send>, &'static str> {
            Ok(Box::new(MemDb { db: HashMap::new() }) as Box<dyn Db + Send>)
        }
    }

    pub fn new_driver() -> Box<dyn Driver> {
        Box::new(MemDriver {})
    }

    #[test]
    fn test_get_put() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

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
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(false));
    }

    #[test]
    fn test_batch() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

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
}
