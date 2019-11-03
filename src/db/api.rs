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

pub struct KeyList {
    pub keys: Vec<Vec<u8>>,
    pub list_end: bool,
}

pub struct IterOptions {
    pub start_key: Option<Vec<u8>>,
    pub prefix: Option<Vec<u8>>,
}

impl IterOptions {
    pub fn new() -> IterOptions {
        IterOptions {
            start_key: None,
            prefix: None,
        }
    }

    pub fn start(&mut self, key: &[u8]) -> &mut IterOptions {
        self.start_key = Some(key.to_vec());

        self
    }

    pub fn prefix(&mut self, prefix: &[u8]) -> &mut IterOptions {
        self.prefix = Some(prefix.to_vec());

        self
    }
}

pub struct DbStat {
    pub n_records: u64,
}

pub const MAX_ITER_KEYS: usize = 1000;

pub trait Db {
    fn apply_batch(&mut self, batch: &Batch) -> Result<bool, &'static str>;
    fn clear(&mut self) -> Result<bool, &'static str>;
    fn del(&mut self, key: &[u8]) -> Result<bool, &'static str>;
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str>;
    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<bool, &'static str>;
    fn iter_keys(&self, opts: IterOptions) -> Result<KeyList, &'static str>;
    fn stat(&self) -> Result<DbStat, &'static str>;
}

pub trait Driver {
    fn start_db(&self, cfg: Config) -> Result<Box<dyn Db + Send>, &'static str>;
}

pub struct ConfigBuilder {
    pub path: Option<String>,
    pub read_only: Option<bool>,
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
        fn clear(&mut self) -> Result<bool, &'static str> {
            self.db.clear();
            Ok(true)
        }

        fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
            match self.db.get(key) {
                None => Ok(None),
                Some(val) => Ok(Some(val.to_vec())),
            }
        }

        fn stat(&self) -> Result<DbStat, &'static str> {
            Ok(DbStat {
                n_records: self.db.len() as u64,
            })
        }

        fn iter_keys(&self, opts: IterOptions) -> Result<KeyList, &'static str> {
            let mut key_list = KeyList {
                keys: Vec::new(),
                list_end: true,
            };

            let prefix: Vec<u8> = match opts.prefix {
                None => Vec::new(),
                Some(value) => value,
            };
            let pfx_len = prefix.len();

            let start_key: Vec<u8> = match opts.start_key {
                None => Vec::new(),
                Some(value) => value,
            };
            let have_start_key: bool = start_key.len() > 0;

            let mut capture = !have_start_key;
            for key in self.db.keys() {
                // handle prefix-only iteration; skip if no match
                if pfx_len > 0 {
                    if key.len() < pfx_len || prefix != &key[0..pfx_len] {
                        continue;
                    }
                }

                // initialize iteration
                if !capture {
                    if start_key == &key[0..] {
                        capture = true;
                        // don't push this key; caller is passing
                        // last key seen in their previous iter()
                    }

                // continue iteration
                } else {
                    key_list.keys.push(key.clone());
                }

                if key_list.keys.len() >= MAX_ITER_KEYS {
                    key_list.list_end = false;
                    break;
                }
            }

            Ok(key_list)
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

    #[test]
    fn test_clear() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.put(b"age", b"25"), Ok(true));
        assert_eq!(db.get(b"name"), Ok(Some(Vec::from("alan"))));
        assert_eq!(db.clear(), Ok(true));
        assert_eq!(db.get(b"name"), Ok(None));
        assert_eq!(db.get(b"age"), Ok(None));
    }

    #[test]
    fn test_stat() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        assert_eq!(db.put(b"name1", b"alan"), Ok(true));
        assert_eq!(db.put(b"age1", b"25"), Ok(true));
        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(true));
        assert_eq!(db.del(b"name"), Ok(false));

        let st = db.stat().unwrap();
        assert_eq!(st.n_records, 2);
    }

    #[test]
    fn test_iter() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        // iterate empty list
        let key_list_res = db.iter_keys(IterOptions::new());
        assert_eq!(key_list_res.is_err(), false);

        let mut key_list = key_list_res.unwrap();
        assert_eq!(key_list.list_end, true);

        key_list.keys.sort();
        assert_eq!(key_list.keys.len(), 0);

        // iterate small list
        assert_eq!(db.put(b"name", b"alan"), Ok(true));
        assert_eq!(db.put(b"age", b"25"), Ok(true));

        let key_list_res = db.iter_keys(IterOptions::new());
        assert_eq!(key_list_res.is_err(), false);

        let mut key_list = key_list_res.unwrap();
        assert_eq!(key_list.list_end, true);

        key_list.keys.sort();
        assert_eq!(key_list.keys.len(), 2);
        assert_eq!(key_list.keys[0], b"age");
        assert_eq!(key_list.keys[1], b"name");
    }

    #[test]
    fn test_iter_prefix() {
        let db_config = ConfigBuilder::new()
            .path("/dev/null".to_string())
            .read_only(false)
            .build();

        let driver = new_driver();

        let mut db = driver.start_db(db_config).unwrap();

        // iterate small list
        assert_eq!(db.put(b"2018/name", b"alan"), Ok(true));
        assert_eq!(db.put(b"2018/bame", b"alan"), Ok(true));
        assert_eq!(db.put(b"2019/fame", b"alan"), Ok(true));
        assert_eq!(db.put(b"2019/lame", b"alan"), Ok(true));
        assert_eq!(db.put(b"2019/game", b"alan"), Ok(true));
        assert_eq!(db.put(b"2020/tame", b"alan"), Ok(true));
        assert_eq!(db.put(b"age", b"25"), Ok(true));

        let key_list_res = db.iter_keys(IterOptions::new());
        assert_eq!(key_list_res.is_err(), false);

        let key_list = key_list_res.unwrap();
        assert_eq!(key_list.list_end, true);
        assert_eq!(key_list.keys.len(), 7);

        // iterate with prefix matching
        let mut opts = IterOptions::new();
        opts.prefix(b"2019/");

        let key_list_res = db.iter_keys(opts);
        assert_eq!(key_list_res.is_err(), false);

        let mut key_list = key_list_res.unwrap();
        assert_eq!(key_list.list_end, true);
        assert_eq!(key_list.keys.len(), 3);

        key_list.keys.sort();
        assert_eq!(
            String::from_utf8_lossy(&key_list.keys[0]),
            String::from("2019/fame")
        );
        assert_eq!(
            String::from_utf8_lossy(&key_list.keys[1]),
            String::from("2019/game")
        );
        assert_eq!(
            String::from_utf8_lossy(&key_list.keys[2]),
            String::from("2019/lame")
        );
    }
}
