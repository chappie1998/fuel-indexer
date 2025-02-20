mod database;
pub mod executor;
pub mod ffi;
mod service;

pub use database::Database;
pub use executor::{Executor, IndexEnv, NativeIndexExecutor, WasmIndexExecutor};
pub use fuel_indexer_database::IndexerDatabaseError;
pub use fuel_indexer_lib::{
    config::IndexerConfig,
    manifest::{Manifest, Module},
};
pub use fuel_indexer_schema::{db::IndexerSchemaError, FtColumn};
use fuel_indexer_types::abi::BlockData;
pub use fuel_types::{Address, ContractId};
pub use service::IndexerService;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasmer::{ExportError, HostEnvInitError, InstantiationError, RuntimeError};

pub type IndexerResult<T> = core::result::Result<T, IndexerError>;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Compiler error: {0:#?}")]
    CompileError(#[from] wasmer::CompileError),
    #[error("Error from sqlx: {0:#?}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Error instantiating wasm interpreter: {0:#?}")]
    InstantiationError(#[from] InstantiationError),
    #[error("Error finding exported symbol: {0:#?}")]
    ExportError(#[from] ExportError),
    #[error("Error executing function: {0:#?}")]
    RuntimeError(#[from] RuntimeError),
    #[error("Could not initialize host environment: {0:#?}")]
    HostEnvInitError(#[from] HostEnvInitError),
    #[error("IO Error: {0:#?}")]
    IoError(#[from] std::io::Error),
    #[error("FFI Error {0:?}")]
    FFIError(#[from] ffi::FFIError),
    #[error("Missing handler")]
    MissingHandler,
    #[error("Indexer transaction error {0:?}")]
    TxError(#[from] crate::executor::TxError),
    #[error("Database error {0:?}")]
    DatabaseError(#[from] IndexerDatabaseError),
    #[error("Invalid address {0:?}")]
    InvalidAddress(#[from] std::net::AddrParseError),
    #[error("Join Error {0:?}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Error initializing executor")]
    ExecutorInitError,
    #[error("Error executing handler")]
    HandlerError,
    #[error("Invalid port {0:?}")]
    InvalidPortNumber(#[from] core::num::ParseIntError),
    #[error("No transaction is open.")]
    NoTransactionError,
    #[error("Unknown error")]
    Unknown,
    #[error("Indexer schema error: {0:?}")]
    SchemaError(#[from] IndexerSchemaError),
}

#[derive(Serialize, Deserialize)]
pub enum IndexerResponse {
    Blocks(Vec<BlockData>),
    Object(Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub enum IndexerRequest {
    GetObject(u64, u64),
    PutObject(u64, Vec<u8>, Vec<FtColumn>),
    Commit,
}
