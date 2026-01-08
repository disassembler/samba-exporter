use std::path::Path;
use trivialdb::{Flags, Tdb, O_RDONLY};

pub fn count_records(path_str: &str) -> i64 {
    let path = Path::new(path_str);
    if !path.exists() {
        return 0;
    }

    match Tdb::open(path, None, Flags::default(), O_RDONLY, 0) {
        Some(tdb) => tdb.iter().count() as i64,
        None => 0,
    }
}
