use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr;

use crate::error::{LocalAIRouterError, Result};

#[allow(non_camel_case_types)]
type sqlite3 = c_void;
#[allow(non_camel_case_types)]
type sqlite3_stmt = c_void;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_TEXT: c_int = 3;
const SQLITE_BLOB: c_int = 4;
const SQLITE_NULL: c_int = 5;

const SQLITE_OPEN_READWRITE: c_int = 0x0000_0002;
const SQLITE_OPEN_CREATE: c_int = 0x0000_0004;
const SQLITE_OPEN_FULLMUTEX: c_int = 0x0001_0000;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        pp_db: *mut *mut sqlite3,
        flags: c_int,
        z_vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    fn sqlite3_exec(
        db: *mut sqlite3,
        sql: *const c_char,
        callback: Option<
            unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char, *mut *mut c_char) -> c_int,
        >,
        arg: *mut c_void,
        errmsg: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_free(ptr: *mut c_void);
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        sql: *const c_char,
        n_byte: c_int,
        stmt: *mut *mut sqlite3_stmt,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_blob(
        stmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_void,
        len: c_int,
        destructor: unsafe extern "C" fn(*mut c_void),
    ) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut sqlite3_stmt, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_null(stmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_char,
        len: c_int,
        destructor: unsafe extern "C" fn(*mut c_void),
    ) -> c_int;
    fn sqlite3_column_count(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_name(stmt: *mut sqlite3_stmt, index: c_int) -> *const c_char;
    fn sqlite3_column_type(stmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_column_int64(stmt: *mut sqlite3_stmt, index: c_int) -> i64;
    fn sqlite3_column_text(stmt: *mut sqlite3_stmt, index: c_int) -> *const u8;
    fn sqlite3_column_blob(stmt: *mut sqlite3_stmt, index: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_step(stmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_finalize(stmt: *mut sqlite3_stmt) -> c_int;
}

#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqlValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_blob(&self) -> Option<&[u8]> {
        match self {
            Self::Blob(value) => Some(value.as_slice()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    columns: BTreeMap<String, SqlValue>,
}

impl Row {
    pub fn get_text(&self, name: &str) -> Result<String> {
        self.columns
            .get(name)
            .and_then(SqlValue::as_text)
            .map(ToOwned::to_owned)
            .ok_or_else(|| LocalAIRouterError::Sqlite(format!("missing text column `{name}`")))
    }

    pub fn get_optional_text(&self, name: &str) -> Result<Option<String>> {
        match self.columns.get(name) {
            Some(SqlValue::Null) | None => Ok(None),
            Some(SqlValue::Text(value)) => Ok(Some(value.clone())),
            _ => Err(LocalAIRouterError::Sqlite(format!(
                "column `{name}` is not text"
            ))),
        }
    }

    pub fn get_i64(&self, name: &str) -> Result<i64> {
        self.columns
            .get(name)
            .and_then(SqlValue::as_i64)
            .ok_or_else(|| LocalAIRouterError::Sqlite(format!("missing integer column `{name}`")))
    }

    pub fn get_optional_i64(&self, name: &str) -> Result<Option<i64>> {
        match self.columns.get(name) {
            Some(SqlValue::Null) | None => Ok(None),
            Some(SqlValue::Integer(value)) => Ok(Some(*value)),
            _ => Err(LocalAIRouterError::Sqlite(format!(
                "column `{name}` is not integer"
            ))),
        }
    }

    pub fn get_blob(&self, name: &str) -> Result<Vec<u8>> {
        self.columns
            .get(name)
            .and_then(SqlValue::as_blob)
            .map(ToOwned::to_owned)
            .ok_or_else(|| LocalAIRouterError::Sqlite(format!("missing blob column `{name}`")))
    }
}

pub struct Connection {
    raw: *mut sqlite3,
}

unsafe impl Send for Connection {}

impl Connection {
    pub fn open(path: &Path) -> Result<Self> {
        let path_c = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| LocalAIRouterError::Sqlite("database path contains NUL byte".into()))?;
        let mut raw = ptr::null_mut();
        let rc = unsafe {
            sqlite3_open_v2(
                path_c.as_ptr(),
                &mut raw,
                SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE | SQLITE_OPEN_FULLMUTEX,
                ptr::null(),
            )
        };
        if rc != SQLITE_OK {
            let message = if raw.is_null() {
                "failed to open sqlite database".to_string()
            } else {
                let message = error_message(raw);
                unsafe {
                    sqlite3_close(raw);
                }
                message
            };
            return Err(LocalAIRouterError::Sqlite(message));
        }
        let connection = Self { raw };
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        Ok(connection)
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let sql_c = CString::new(sql)
            .map_err(|_| LocalAIRouterError::Sqlite("SQL contains NUL byte".into()))?;
        let mut error_ptr: *mut c_char = ptr::null_mut();
        let rc = unsafe {
            sqlite3_exec(
                self.raw,
                sql_c.as_ptr(),
                None,
                ptr::null_mut(),
                &mut error_ptr,
            )
        };
        if rc != SQLITE_OK {
            let message = if error_ptr.is_null() {
                error_message(self.raw)
            } else {
                let msg = unsafe { CStr::from_ptr(error_ptr).to_string_lossy().into_owned() };
                unsafe {
                    sqlite3_free(error_ptr.cast());
                }
                msg
            };
            return Err(LocalAIRouterError::Sqlite(message));
        }
        Ok(())
    }

    pub fn execute(&self, sql: &str, params: &[SqlValue]) -> Result<()> {
        let stmt = self.prepare(sql)?;
        bind_all(stmt, params)?;
        let rc = unsafe { sqlite3_step(stmt) };
        finalize(stmt);
        if rc != SQLITE_DONE {
            return Err(LocalAIRouterError::Sqlite(error_message(self.raw)));
        }
        Ok(())
    }

    pub fn query(&self, sql: &str, params: &[SqlValue]) -> Result<Vec<Row>> {
        let stmt = self.prepare(sql)?;
        bind_all(stmt, params)?;
        let column_count = unsafe { sqlite3_column_count(stmt) };
        let mut rows = Vec::new();
        loop {
            match unsafe { sqlite3_step(stmt) } {
                SQLITE_ROW => {
                    let mut columns = BTreeMap::new();
                    for index in 0..column_count {
                        let name = unsafe {
                            CStr::from_ptr(sqlite3_column_name(stmt, index))
                                .to_string_lossy()
                                .into_owned()
                        };
                        let value = match unsafe { sqlite3_column_type(stmt, index) } {
                            SQLITE_INTEGER => {
                                SqlValue::Integer(unsafe { sqlite3_column_int64(stmt, index) })
                            }
                            SQLITE_TEXT => SqlValue::Text(read_text(stmt, index)),
                            SQLITE_BLOB => SqlValue::Blob(read_blob(stmt, index)),
                            SQLITE_NULL => SqlValue::Null,
                            _ => SqlValue::Text(read_text(stmt, index)),
                        };
                        columns.insert(name, value);
                    }
                    rows.push(Row { columns });
                }
                SQLITE_DONE => break,
                _ => {
                    let message = error_message(self.raw);
                    finalize(stmt);
                    return Err(LocalAIRouterError::Sqlite(message));
                }
            }
        }
        finalize(stmt);
        Ok(rows)
    }

    fn prepare(&self, sql: &str) -> Result<*mut sqlite3_stmt> {
        let sql_c = CString::new(sql)
            .map_err(|_| LocalAIRouterError::Sqlite("SQL contains NUL byte".into()))?;
        let mut stmt = ptr::null_mut();
        let rc =
            unsafe { sqlite3_prepare_v2(self.raw, sql_c.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        if rc != SQLITE_OK {
            return Err(LocalAIRouterError::Sqlite(error_message(self.raw)));
        }
        Ok(stmt)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.raw);
        }
    }
}

fn bind_all(stmt: *mut sqlite3_stmt, params: &[SqlValue]) -> Result<()> {
    for (idx, value) in params.iter().enumerate() {
        let index = (idx + 1) as c_int;
        let rc = match value {
            SqlValue::Null => unsafe { sqlite3_bind_null(stmt, index) },
            SqlValue::Integer(value) => unsafe { sqlite3_bind_int64(stmt, index, *value) },
            SqlValue::Text(value) => {
                let cstr = CString::new(value.as_str()).map_err(|_| {
                    LocalAIRouterError::Sqlite("text parameter contains NUL byte".into())
                })?;
                unsafe {
                    sqlite3_bind_text(
                        stmt,
                        index,
                        cstr.as_ptr(),
                        value.len() as c_int,
                        sqlite_transient(),
                    )
                }
            }
            SqlValue::Blob(value) => unsafe {
                sqlite3_bind_blob(
                    stmt,
                    index,
                    value.as_ptr().cast(),
                    value.len() as c_int,
                    sqlite_transient(),
                )
            },
        };
        if rc != SQLITE_OK {
            return Err(LocalAIRouterError::Sqlite(
                "failed to bind sqlite parameter".into(),
            ));
        }
    }
    Ok(())
}

fn read_text(stmt: *mut sqlite3_stmt, index: c_int) -> String {
    let pointer = unsafe { sqlite3_column_text(stmt, index) };
    if pointer.is_null() {
        return String::new();
    }
    let len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
    let bytes = unsafe { std::slice::from_raw_parts(pointer, len) };
    String::from_utf8_lossy(bytes).into_owned()
}

fn read_blob(stmt: *mut sqlite3_stmt, index: c_int) -> Vec<u8> {
    let pointer = unsafe { sqlite3_column_blob(stmt, index) };
    if pointer.is_null() {
        return Vec::new();
    }
    let len = unsafe { sqlite3_column_bytes(stmt, index) } as usize;
    unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), len) }.to_vec()
}

fn error_message(raw: *mut sqlite3) -> String {
    unsafe { CStr::from_ptr(sqlite3_errmsg(raw)) }
        .to_string_lossy()
        .into_owned()
}

fn finalize(stmt: *mut sqlite3_stmt) {
    unsafe {
        sqlite3_finalize(stmt);
    }
}

fn sqlite_transient() -> unsafe extern "C" fn(*mut c_void) {
    unsafe { std::mem::transmute::<isize, unsafe extern "C" fn(*mut c_void)>(-1) }
}
