use upload_service::core::{ entities::{UploadedFileCreate, UploadedFile }, repository::Repository };
use sqlx::{ query_as, query_scalar };
use anyhow::Error;
use uuid::Uuid;

use super::PostgresRepository;

impl Repository for PostgresRepository {
	async fn get_uploaded_file(&self, id: &str) -> Result<Option<UploadedFile>, Error> {
		Ok(query_as!(UploadedFile, "SELECT * FROM uploads WHERE id = $1", id)
		.fetch_optional(&self.pool)
		.await
		.map_err(|e| Error::new(e))?)
	    
	}
	async fn insert_uploaded_file(&self, file: UploadedFileCreate) -> Result<String, Error> {
	    Ok(query_scalar!("INSERT INTO uploads 
	    (id, filename, mime_type, filepath, uploader_id, uploaded_at)
	    VALUES ($1, $2, $3, $4, $5, $6)
	    RETURNING id",
		Uuid::new_v4().to_string(), file.filename, file.mime_type, file.filepath, file.uploader_id, file.uploaded_at)
		.fetch_one(&self.pool)
		.await
		.map_err(|e| Error::new(e))?)
	}
}


