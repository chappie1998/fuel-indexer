use crate::{ffi, IndexerError, IndexerResult, Manifest};
use fuel_indexer_database::{queries, IndexerConnection, IndexerConnectionPool};
use fuel_indexer_schema::{utils::IdCol, FtColumn};
use std::collections::HashMap;
use wasmer::Instance;

/// Database for an executor instance, with schema info.
#[derive(Debug)]
pub struct Database {
    pub pool: IndexerConnectionPool,
    stashed: Option<IndexerConnection>,
    pub namespace: String,
    pub version: String,
    pub schema: HashMap<String, Vec<String>>,
    pub tables: HashMap<u64, String>,
}

// Hmm, TODO Mutecks instedddD
unsafe impl Sync for Database {}
unsafe impl Send for Database {}

impl Database {
    pub async fn new(db_conn: &str) -> IndexerResult<Database> {
        let pool = IndexerConnectionPool::connect(db_conn).await?;

        Ok(Database {
            pool,
            stashed: None,
            namespace: Default::default(),
            version: Default::default(),
            schema: Default::default(),
            tables: Default::default(),
        })
    }

    pub async fn start_transaction(&mut self) -> IndexerResult<usize> {
        let mut conn = self.pool.acquire().await?;
        let result = queries::execute_query(&mut conn, "BEGIN".into()).await?;

        self.stashed = Some(conn);

        Ok(result)
    }

    pub async fn commit_transaction(&mut self) -> IndexerResult<usize> {
        let mut conn = self
            .stashed
            .take()
            .ok_or(IndexerError::NoTransactionError)?;
        Ok(queries::execute_query(&mut conn, "COMMIT".into()).await?)
    }

    pub async fn revert_transaction(&mut self) -> IndexerResult<usize> {
        let mut conn = self
            .stashed
            .take()
            .ok_or(IndexerError::NoTransactionError)?;
        Ok(queries::execute_query(&mut conn, "ROLLBACK".into()).await?)
    }

    // FIXME: Upsert requires entities that have 1+ fields
    fn upsert_query(
        &self,
        table: &str,
        columns: &[String],
        inserts: Vec<String>,
        updates: Vec<String>,
    ) -> String {
        let sql_table = self.pool.database_type().table_name(&self.namespace, table);

        // FIXME: We have hard-coded the concept of an 'id' field here <(-_-<)
        format!(
            "INSERT INTO {}
                ({})
             VALUES
                ({}, $1)
             ON CONFLICT(id)
             DO UPDATE SET {}",
            sql_table,
            columns.join(", "),
            inserts.join(", "),
            updates.join(", "),
        )
    }

    fn get_query(&self, table: &str, object_id: u64) -> String {
        // FIXME: We have hard-coded the concept of an 'id' field here <(-_-<)
        let sql_table = self.pool.database_type().table_name(&self.namespace, table);
        format!("SELECT object from {} where id = {}", sql_table, object_id)
    }

    pub async fn put_object(
        &mut self,
        type_id: u64,
        columns: Vec<FtColumn>,
        bytes: Vec<u8>,
    ) {
        let table = self.tables.get(&type_id).unwrap_or_else(|| {
            panic!(
                "TypeId({}) not found in tables: {:?}. Is your WASM module up-to-date?",
                type_id, self.tables
            )
        });
        let inserts: Vec<_> = columns.iter().map(|col| col.query_fragment()).collect();
        let updates: Vec<_> = self.schema[table]
            .iter()
            .zip(columns.iter())
            .filter_map(|(colname, value)| {
                if colname == &IdCol::to_lowercase_string() {
                    None
                } else {
                    Some(format!("{} = {}", colname, value.query_fragment()))
                }
            })
            .collect();

        let columns = self.schema[table].clone();

        let query_text = self.upsert_query(table, &columns, inserts, updates);

        let conn = self
            .stashed
            .as_mut()
            .expect("No transaction has been opened.");
        let query = queries::put_object(conn, query_text, bytes).await;

        query.expect("Query failed.");
    }

    pub async fn get_object(&mut self, type_id: u64, object_id: u64) -> Option<Vec<u8>> {
        let table = &self.tables[&type_id];
        let query = self.get_query(table, object_id);

        let conn = self
            .stashed
            .as_mut()
            .expect("No transaction has been opened.");
        match queries::get_object(conn, query).await {
            Ok(object) => Some(object),
            Err(sqlx::Error::RowNotFound) => None,
            e => {
                panic!("Error getting object: {:?}.", e);
            }
        }
    }

    pub async fn load_schema_native(&mut self, manifest: Manifest) -> IndexerResult<()> {
        self.namespace = manifest.namespace;

        let mut conn = self.pool.acquire().await?;
        self.version = queries::type_id_latest(&mut conn, &self.namespace).await?;

        let results =
            queries::columns_get_schema(&mut conn, &self.namespace, &self.version)
                .await?;

        for column in results {
            let table = &column.table_name;

            self.tables
                .entry(column.type_id as u64)
                .or_insert_with(|| table.to_string());

            let columns = self
                .schema
                .entry(table.to_string())
                .or_insert_with(Vec::new);

            columns.push(column.column_name);
        }

        Ok(())
    }

    pub async fn load_schema_wasm(&mut self, instance: &Instance) -> IndexerResult<()> {
        self.namespace = ffi::get_namespace(instance)?;
        self.version = ffi::get_version(instance)?;

        let mut conn = self.pool.acquire().await?;
        let results =
            queries::columns_get_schema(&mut conn, &self.namespace, &self.version)
                .await?;

        for column in results {
            let table = &column.table_name;

            self.tables
                .entry(column.type_id as u64)
                .or_insert_with(|| table.to_string());

            let columns = self
                .schema
                .entry(table.to_string())
                .or_insert_with(Vec::new);

            columns.push(column.column_name);
        }

        Ok(())
    }
}
