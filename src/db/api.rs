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
