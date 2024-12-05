use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub async fn create_root_dir(path: &str) -> tokio::io::Result<()> {
    if !Path::new(path).exists() {
        tokio::fs::create_dir_all(path).await?;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o775);
        tokio::fs::set_permissions(path, permissions).await?;
    }
    Ok(())
}