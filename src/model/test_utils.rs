use std::sync::{Mutex, MutexGuard};

use crate::model::connection::Connection;

pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap()
}

pub(crate) fn reset_db_pool() {
    Connection::reset_db_pool_for_tests();
}
