use std::{fmt, io::Cursor, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use tokio::{fs, fs::OpenOptions, io::AsyncWriteExt};
use uuid::Uuid;

use crate::application::plugin_protocol::is_valid_login_background_asset_id;

const LOGIN_BACKGROUND_ASSET_DIRECTORY: &str = "login-background-assets";
pub const MAX_LOGIN_BACKGROUND_ASSET_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_LOGIN_BACKGROUND_ASSET_PIXELS: u64 = 20_000_000;

#[derive(Clone)]
pub struct LoginBackgroundAssetStore {
    directory: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredLoginBackgroundAsset {
    pub asset_id: String,
    pub content_type: &'static str,
    pub created: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedLoginBackgroundAsset {
    pub asset_id: String,
    pub content_type: &'static str,
    pub etag: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LoginBackgroundAssetError {
    InvalidAssetId,
    InvalidContent,
    TooLarge,
    DimensionsTooLarge,
    InvalidPath,
    Io(std::io::ErrorKind),
}

impl LoginBackgroundAssetStore {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            directory: config_dir.join(LOGIN_BACKGROUND_ASSET_DIRECTORY),
        }
    }

    pub async fn store(
        &self,
        bytes: &[u8],
    ) -> Result<StoredLoginBackgroundAsset, LoginBackgroundAssetError> {
        if bytes.len() > MAX_LOGIN_BACKGROUND_ASSET_BYTES {
            return Err(LoginBackgroundAssetError::TooLarge);
        }
        let content_type = validate_image(bytes)?;
        self.ensure_directory().await?;

        let asset_id = asset_id_for_bytes(bytes);
        let path = self.path_for_asset(&asset_id)?;
        let created = match fs::symlink_metadata(&path).await {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(LoginBackgroundAssetError::InvalidPath);
            }
            Ok(metadata) if metadata.len() == bytes.len() as u64 => {
                let existing = fs::read(&path)
                    .await
                    .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
                if existing == bytes {
                    return Ok(StoredLoginBackgroundAsset {
                        asset_id,
                        content_type,
                        created: false,
                    });
                }
                write_atomically(&path, bytes).await?;
                true
            }
            Ok(_) => {
                write_atomically(&path, bytes).await?;
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_atomically(&path, bytes).await?;
                true
            }
            Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
        };

        Ok(StoredLoginBackgroundAsset {
            asset_id,
            content_type,
            created,
        })
    }

    pub async fn load(
        &self,
        asset_id: &str,
    ) -> Result<Option<LoadedLoginBackgroundAsset>, LoginBackgroundAssetError> {
        let path = self.path_for_asset(asset_id)?;
        if !self.directory_exists().await? {
            return Ok(None);
        }
        let metadata = match fs::symlink_metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LoginBackgroundAssetError::InvalidPath);
        }
        if metadata.len() > MAX_LOGIN_BACKGROUND_ASSET_BYTES as u64 {
            return Err(LoginBackgroundAssetError::TooLarge);
        }
        let bytes = fs::read(&path)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        if bytes.len() > MAX_LOGIN_BACKGROUND_ASSET_BYTES {
            return Err(LoginBackgroundAssetError::TooLarge);
        }
        let content_type = validate_image(&bytes)?;
        if asset_id_for_bytes(&bytes) != asset_id {
            return Err(LoginBackgroundAssetError::InvalidContent);
        }
        Ok(Some(LoadedLoginBackgroundAsset {
            asset_id: asset_id.to_owned(),
            content_type,
            etag: format!("\"{asset_id}\""),
            bytes,
        }))
    }

    pub async fn remove(&self, asset_id: &str) -> Result<(), LoginBackgroundAssetError> {
        let path = self.path_for_asset(asset_id)?;
        if !self.directory_exists().await? {
            return Ok(());
        }
        let metadata = match fs::symlink_metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LoginBackgroundAssetError::InvalidPath);
        }
        fs::remove_file(path)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))
    }

    pub async fn prune_except(&self, asset_id: &str) -> Result<(), LoginBackgroundAssetError> {
        let keep_digest = asset_digest(asset_id)?;
        if !self.directory_exists().await? {
            return Ok(());
        }

        let mut entries = fs::read_dir(&self.directory)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?
        {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let is_asset = is_hex_digest(name);
            let is_temporary = name.starts_with(".upload-") && name.ends_with(".tmp");
            if name == keep_digest || (!is_asset && !is_temporary) {
                continue;
            }
            let file_type = entry
                .file_type()
                .await
                .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
            if file_type.is_dir() {
                continue;
            }
            fs::remove_file(entry.path())
                .await
                .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        }
        Ok(())
    }

    async fn ensure_directory(&self) -> Result<(), LoginBackgroundAssetError> {
        match fs::symlink_metadata(&self.directory).await {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = self
                    .directory
                    .parent()
                    .ok_or(LoginBackgroundAssetError::InvalidPath)?;
                fs::create_dir_all(parent)
                    .await
                    .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
                match fs::create_dir(&self.directory).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
                }
            }
            Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
        }
        if !self.directory_exists().await? {
            return Err(LoginBackgroundAssetError::InvalidPath);
        }
        #[cfg(unix)]
        fs::set_permissions(&self.directory, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        Ok(())
    }

    async fn directory_exists(&self) -> Result<bool, LoginBackgroundAssetError> {
        let metadata = fs::symlink_metadata(&self.directory).await;
        let metadata = match metadata {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(LoginBackgroundAssetError::Io(error.kind())),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LoginBackgroundAssetError::InvalidPath);
        }
        Ok(true)
    }

    fn path_for_asset(&self, asset_id: &str) -> Result<PathBuf, LoginBackgroundAssetError> {
        Ok(self.directory.join(asset_digest(asset_id)?))
    }
}

fn asset_digest(asset_id: &str) -> Result<&str, LoginBackgroundAssetError> {
    if !is_valid_login_background_asset_id(asset_id) {
        return Err(LoginBackgroundAssetError::InvalidAssetId);
    }
    asset_id
        .strip_prefix("sha256:")
        .ok_or(LoginBackgroundAssetError::InvalidAssetId)
}

fn asset_id_for_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_image(bytes: &[u8]) -> Result<&'static str, LoginBackgroundAssetError> {
    let format =
        image::guess_format(bytes).map_err(|_| LoginBackgroundAssetError::InvalidContent)?;
    let content_type = match format {
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::WebP => "image/webp",
        _ => return Err(LoginBackgroundAssetError::InvalidContent),
    };
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| LoginBackgroundAssetError::InvalidContent)?;
    validate_dimensions(width, height)?;
    Ok(content_type)
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), LoginBackgroundAssetError> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || pixels > MAX_LOGIN_BACKGROUND_ASSET_PIXELS {
        return Err(LoginBackgroundAssetError::DimensionsTooLarge);
    }
    Ok(())
}

async fn write_atomically(
    path: &std::path::Path,
    bytes: &[u8],
) -> Result<(), LoginBackgroundAssetError> {
    let parent = path
        .parent()
        .ok_or(LoginBackgroundAssetError::InvalidPath)?;
    let temporary = parent.join(format!(".upload-{}.tmp", Uuid::now_v7()));
    let result = async {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        file.write_all(bytes)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        file.sync_all()
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        drop(file);
        #[cfg(unix)]
        fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        fs::rename(&temporary, path)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        let directory = fs::File::open(parent)
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))?;
        directory
            .sync_all()
            .await
            .map_err(|error| LoginBackgroundAssetError::Io(error.kind()))
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    result
}

impl fmt::Display for LoginBackgroundAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidAssetId => "login background image reference is invalid",
            Self::InvalidContent => "login background image content is invalid",
            Self::TooLarge => "login background image exceeds the size limit",
            Self::DimensionsTooLarge => "login background image dimensions exceed the limit",
            Self::InvalidPath => "login background image storage path is invalid",
            Self::Io(_) => "login background image storage failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LoginBackgroundAssetError {}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat};
    use tempfile::tempdir;

    use super::{
        LoginBackgroundAssetError, LoginBackgroundAssetStore, MAX_LOGIN_BACKGROUND_ASSET_BYTES,
    };

    fn image_bytes(format: ImageFormat) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        DynamicImage::new_rgb8(4, 3)
            .write_to(&mut output, format)
            .expect("image fixture should encode");
        output.into_inner()
    }

    #[tokio::test]
    async fn stores_and_loads_original_png_bytes_by_opaque_asset_id() {
        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().to_owned());
        let bytes = image_bytes(ImageFormat::Png);

        let stored = store
            .store(&bytes)
            .await
            .expect("valid png should be stored");
        assert!(stored.asset_id.starts_with("sha256:"));
        assert_eq!(stored.asset_id.len(), "sha256:".len() + 64);
        assert_eq!(stored.content_type, "image/png");

        let loaded = store
            .load(&stored.asset_id)
            .await
            .expect("stored image should be loadable")
            .expect("stored image should exist");
        assert_eq!(loaded.bytes, bytes);
        assert_eq!(loaded.content_type, "image/png");
        assert!(loaded.etag.starts_with('"') && loaded.etag.ends_with('"'));
    }

    #[tokio::test]
    async fn rejects_unsupported_or_oversized_uploads_before_writing() {
        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().to_owned());

        assert_eq!(
            store.store(b"<svg></svg>").await,
            Err(LoginBackgroundAssetError::InvalidContent)
        );
        assert_eq!(
            store
                .store(&vec![0; MAX_LOGIN_BACKGROUND_ASSET_BYTES + 1])
                .await,
            Err(LoginBackgroundAssetError::TooLarge)
        );
    }

    #[tokio::test]
    async fn stores_supported_static_raster_formats() {
        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().to_owned());

        for (format, content_type) in [
            (ImageFormat::Jpeg, "image/jpeg"),
            (ImageFormat::Png, "image/png"),
            (ImageFormat::WebP, "image/webp"),
        ] {
            let stored = store
                .store(&image_bytes(format))
                .await
                .expect("supported image format should be stored");
            assert_eq!(stored.content_type, content_type);
        }
    }

    #[tokio::test]
    async fn replacement_cleanup_keeps_only_the_selected_opaque_asset() {
        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().to_owned());
        let first = store
            .store(&image_bytes(ImageFormat::Png))
            .await
            .expect("first image should be stored");
        assert!(first.created);
        assert!(
            !store
                .store(&image_bytes(ImageFormat::Png))
                .await
                .expect("identical content should be idempotent")
                .created
        );
        let second = store
            .store(&image_bytes(ImageFormat::Jpeg))
            .await
            .expect("replacement should be stored");

        store
            .prune_except(&second.asset_id)
            .await
            .expect("replaced images should be pruned");
        assert!(
            store
                .load(&first.asset_id)
                .await
                .expect("load should work")
                .is_none()
        );
        assert!(
            store
                .load(&second.asset_id)
                .await
                .expect("load should work")
                .is_some()
        );
    }

    #[test]
    fn rejects_zero_or_more_than_twenty_megapixel_dimensions() {
        assert_eq!(
            super::validate_dimensions(0, 10),
            Err(LoginBackgroundAssetError::DimensionsTooLarge)
        );
        assert_eq!(
            super::validate_dimensions(5000, 5000),
            Err(LoginBackgroundAssetError::DimensionsTooLarge)
        );
        assert!(super::validate_dimensions(5000, 4000).is_ok());
    }

    #[tokio::test]
    async fn rejects_path_like_asset_ids() {
        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().to_owned());
        assert_eq!(
            store.load("sha256:../../secret").await,
            Err(LoginBackgroundAssetError::InvalidAssetId)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_a_symlinked_asset_directory_without_reading_its_target() {
        use std::os::unix::fs::symlink;

        let temporary = tempdir().expect("temporary directory should be available");
        let config = temporary.path().join("config");
        let outside = temporary.path().join("outside");
        std::fs::create_dir_all(&config).expect("config directory should be created");
        std::fs::create_dir_all(&outside).expect("outside directory should be created");
        std::fs::write(outside.join("sentinel"), b"keep").expect("sentinel should be written");
        symlink(&outside, config.join("login-background-assets"))
            .expect("asset directory symlink should be created");
        let store = LoginBackgroundAssetStore::new(config);
        let image = image_bytes(ImageFormat::Png);
        let asset_id = format!("sha256:{}", "a".repeat(64));

        assert_eq!(
            store.load(&asset_id).await,
            Err(LoginBackgroundAssetError::InvalidPath)
        );
        assert_eq!(
            store.store(&image).await,
            Err(LoginBackgroundAssetError::InvalidPath)
        );
        assert_eq!(
            std::fs::read(outside.join("sentinel")).expect("sentinel should remain"),
            b"keep"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_a_symlinked_asset_file_without_following_it() {
        use std::os::unix::fs::symlink;

        let temporary = tempdir().expect("temporary directory should be available");
        let store = LoginBackgroundAssetStore::new(temporary.path().join("config"));
        let bytes = image_bytes(ImageFormat::Png);
        let stored = store
            .store(&bytes)
            .await
            .expect("valid image should be stored");
        let digest = stored
            .asset_id
            .strip_prefix("sha256:")
            .expect("asset ID should use the SHA-256 prefix");
        let asset_path = store.directory.join(digest);
        store
            .remove(&stored.asset_id)
            .await
            .expect("stored image should be removable for the test");
        let outside = temporary.path().join("outside");
        std::fs::write(&outside, b"must not be read or replaced")
            .expect("outside sentinel should be written");
        symlink(&outside, &asset_path).expect("asset symlink should be created");

        assert_eq!(
            store.load(&stored.asset_id).await,
            Err(LoginBackgroundAssetError::InvalidPath)
        );
        assert_eq!(
            store.store(&bytes).await,
            Err(LoginBackgroundAssetError::InvalidPath)
        );
        assert_eq!(
            std::fs::read(&outside).expect("outside sentinel should remain"),
            b"must not be read or replaced"
        );
    }
}
