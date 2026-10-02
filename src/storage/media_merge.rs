use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredMediaMerge {
    pub(crate) primary_item_id: String,
    pub(crate) library_id: String,
    pub(crate) item_type: String,
    pub(crate) merged_item_ids: Vec<String>,
}

#[derive(Debug)]
struct MergeRootItem {
    id: String,
    library_id: String,
    item_type: String,
    merged_into_item_id: Option<String>,
    removed_at: Option<i64>,
}

#[derive(Debug)]
struct MergeHierarchyItem {
    id: String,
    season_number: Option<i64>,
    episode_number: Option<i64>,
}

impl Database {
    pub(crate) async fn merge_media_items(
        &self,
        primary_item_id: &str,
        item_ids: &[String],
    ) -> Result<StoredMediaMerge, StorageError> {
        if item_ids.len() < 2 {
            return Err(StorageError::Conflict(
                "至少需要选择两个媒体条目".to_owned(),
            ));
        }
        if item_ids.iter().collect::<HashSet<_>>().len() != item_ids.len() {
            return Err(StorageError::Conflict("待合并条目不能重复".to_owned()));
        }
        if !item_ids.iter().any(|item_id| item_id == primary_item_id) {
            return Err(StorageError::Conflict("主条目必须来自已选条目".to_owned()));
        }

        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let mut roots = Vec::with_capacity(item_ids.len());
        for item_id in item_ids {
            let root = self
                .query(
                    "SELECT mi.id, mi.library_id, mi.item_type,
                            mi.merged_into_item_id, mi.removed_at
                     FROM media_items mi
                     JOIN libraries l ON l.id = mi.library_id AND l.is_enabled = 1
                     WHERE mi.id = ?",
                )
                .bind(item_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|source| StorageError::Sqlx {
                    path: self.path.clone(),
                    source,
                })?
                .map(|row| MergeRootItem {
                    id: row.get("id"),
                    library_id: row.get("library_id"),
                    item_type: row.get("item_type"),
                    merged_into_item_id: row.get("merged_into_item_id"),
                    removed_at: row.get("removed_at"),
                });
            let Some(root) = root else {
                return Err(StorageError::Conflict("媒体条目不存在".to_owned()));
            };
            roots.push(root);
        }

        let primary = roots
            .iter()
            .find(|root| root.id == primary_item_id)
            .ok_or_else(|| StorageError::Conflict("主条目不存在".to_owned()))?;
        validate_merge_root(primary, &roots)?;
        let merged_item_ids = roots
            .iter()
            .filter(|root| root.id != primary_item_id)
            .map(|root| root.id.clone())
            .collect::<Vec<_>>();

        let primary_has_default =
            self.query_scalar::<i64>(
                "SELECT EXISTS (
                     SELECT 1 FROM media_sources
                     WHERE item_id = ? AND is_default = 1
                 )",
            )
            .bind(primary_item_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })? != 0;

        if primary.item_type == "MOVIE" {
            for merged_item_id in &merged_item_ids {
                self.merge_sources_in_transaction(
                    &mut transaction,
                    merged_item_id,
                    primary_item_id,
                    primary_has_default,
                )
                .await?;
            }
        } else {
            for merged_item_id in &merged_item_ids {
                self.merge_series_in_transaction(&mut transaction, merged_item_id, primary_item_id)
                    .await?;
            }
        }

        for merged_item_id in &merged_item_ids {
            self.merge_user_item_state_in_transaction(
                &mut transaction,
                merged_item_id,
                primary_item_id,
            )
            .await?;
            self.query(
                "UPDATE media_items
                 SET merged_into_item_id = ?, updated_at = unixepoch()
                 WHERE id = ? AND merged_into_item_id IS NULL",
            )
            .bind(primary_item_id)
            .bind(merged_item_id)
            .execute(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        }

        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(StoredMediaMerge {
            primary_item_id: primary_item_id.to_owned(),
            library_id: primary.library_id.clone(),
            item_type: primary.item_type.clone(),
            merged_item_ids,
        })
    }

    async fn merge_sources_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        source_item_id: &str,
        target_item_id: &str,
        target_has_default: bool,
    ) -> Result<(), StorageError> {
        self.query(
            "UPDATE media_sources
             SET item_id = ?, is_default = CASE WHEN ? = 1 THEN 0 ELSE is_default END,
                 updated_at = unixepoch()
             WHERE item_id = ?",
        )
        .bind(target_item_id)
        .bind(database_flag(target_has_default))
        .bind(source_item_id)
        .execute(&mut **transaction)
        .await
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        self.normalize_default_source_in_transaction(transaction, target_item_id)
            .await
    }

    async fn normalize_default_source_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        item_id: &str,
    ) -> Result<(), StorageError> {
        self.query(
            "UPDATE media_sources
             SET is_default = CASE WHEN id = (
                 SELECT id FROM media_sources
                 WHERE item_id = ?
                 ORDER BY is_default DESC, id
                 LIMIT 1
             ) THEN 1 ELSE 0 END,
                 updated_at = unixepoch()
             WHERE item_id = ?",
        )
        .bind(item_id)
        .bind(item_id)
        .execute(&mut **transaction)
        .await
        .map(|_| ())
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    async fn merge_series_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        source_series_id: &str,
        target_series_id: &str,
    ) -> Result<(), StorageError> {
        let target_seasons = self
            .list_series_seasons_in_transaction(transaction, target_series_id)
            .await?;
        let source_seasons = self
            .list_series_seasons_in_transaction(transaction, source_series_id)
            .await?;
        let episodes_by_season = self
            .list_series_episodes_by_season_in_transaction(
                transaction,
                source_series_id,
                target_series_id,
            )
            .await?;
        for source_season in source_seasons {
            let target_season = source_season
                .season_number
                .and_then(|number| {
                    target_seasons
                        .iter()
                        .find(|season| season.season_number == Some(number))
                })
                .map(|season| season.id.clone());
            let source_episodes = episodes_by_season
                .get(&source_season.id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let Some(target_season_id) = target_season else {
                self.query(
                    "UPDATE media_items
                     SET parent_id = ?, series_id = ?, updated_at = unixepoch()
                     WHERE id = ? AND item_type = 'SEASON'",
                )
                .bind(target_series_id)
                .bind(target_series_id)
                .bind(&source_season.id)
                .execute(&mut **transaction)
                .await
                .map_err(|source| StorageError::Sqlx {
                    path: self.path.clone(),
                    source,
                })?;
                for episode in source_episodes {
                    self.query(
                        "UPDATE media_items
                         SET series_id = ?, updated_at = unixepoch()
                         WHERE id = ? AND item_type = 'EPISODE'",
                    )
                    .bind(target_series_id)
                    .bind(&episode.id)
                    .execute(&mut **transaction)
                    .await
                    .map_err(|source| StorageError::Sqlx {
                        path: self.path.clone(),
                        source,
                    })?;
                }
                continue;
            };

            self.merge_user_item_state_in_transaction(
                transaction,
                &source_season.id,
                &target_season_id,
            )
            .await?;
            self.query(
                "UPDATE media_items
                 SET merged_into_item_id = ?, updated_at = unixepoch()
                 WHERE id = ?",
            )
            .bind(&target_season_id)
            .bind(&source_season.id)
            .execute(&mut **transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;

            let target_episodes = episodes_by_season
                .get(&target_season_id)
                .map(Vec::as_slice)
                .unwrap_or_default();
            for source_episode in source_episodes {
                let target_episode = source_episode
                    .episode_number
                    .and_then(|number| {
                        target_episodes
                            .iter()
                            .find(|episode| episode.episode_number == Some(number))
                    })
                    .map(|episode| episode.id.clone());
                if let Some(target_episode_id) = target_episode {
                    self.merge_sources_in_transaction(
                        transaction,
                        &source_episode.id,
                        &target_episode_id,
                        false,
                    )
                    .await?;
                    self.merge_user_item_state_in_transaction(
                        transaction,
                        &source_episode.id,
                        &target_episode_id,
                    )
                    .await?;
                    self.query(
                        "UPDATE media_items
                         SET merged_into_item_id = ?, updated_at = unixepoch()
                         WHERE id = ?",
                    )
                    .bind(&target_episode_id)
                    .bind(&source_episode.id)
                    .execute(&mut **transaction)
                    .await
                    .map_err(|source| StorageError::Sqlx {
                        path: self.path.clone(),
                        source,
                    })?;
                } else {
                    self.query(
                        "UPDATE media_items
                         SET parent_id = ?, series_id = ?, updated_at = unixepoch()
                         WHERE id = ? AND item_type = 'EPISODE'",
                    )
                    .bind(&target_season_id)
                    .bind(target_series_id)
                    .bind(&source_episode.id)
                    .execute(&mut **transaction)
                    .await
                    .map_err(|source| StorageError::Sqlx {
                        path: self.path.clone(),
                        source,
                    })?;
                }
            }
        }
        Ok(())
    }

    async fn list_series_seasons_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        series_id: &str,
    ) -> Result<Vec<MergeHierarchyItem>, StorageError> {
        self.query(
            "SELECT id, season_number, NULL AS episode_number
             FROM media_items
             WHERE item_type = 'SEASON' AND removed_at IS NULL
               AND merged_into_item_id IS NULL
               AND (series_id = ? OR parent_id = ?)
             ORDER BY season_number, id",
        )
        .bind(series_id)
        .bind(series_id)
        .fetch_all(&mut **transaction)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| MergeHierarchyItem {
                    id: row.get("id"),
                    season_number: row.get("season_number"),
                    episode_number: None,
                })
                .collect()
        })
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    async fn list_series_episodes_by_season_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        source_series_id: &str,
        target_series_id: &str,
    ) -> Result<HashMap<String, Vec<MergeHierarchyItem>>, StorageError> {
        self.query(
            "WITH merge_seasons AS (
                 SELECT id
                 FROM media_items
                 WHERE item_type = 'SEASON' AND removed_at IS NULL
                   AND merged_into_item_id IS NULL
                   AND (series_id IN (?, ?) OR parent_id IN (?, ?))
             )
             SELECT episode.id, episode.parent_id, episode.episode_number
             FROM media_items episode
             JOIN merge_seasons season ON season.id = episode.parent_id
             WHERE episode.item_type = 'EPISODE' AND episode.removed_at IS NULL
               AND episode.merged_into_item_id IS NULL
             ORDER BY episode.parent_id, episode.episode_number, episode.id",
        )
        .bind(source_series_id)
        .bind(target_series_id)
        .bind(source_series_id)
        .bind(target_series_id)
        .fetch_all(&mut **transaction)
        .await
        .map(|rows| {
            let mut episodes_by_season = HashMap::<String, Vec<MergeHierarchyItem>>::new();
            for row in rows {
                episodes_by_season
                    .entry(row.get("parent_id"))
                    .or_default()
                    .push(MergeHierarchyItem {
                        id: row.get("id"),
                        season_number: None,
                        episode_number: row.get("episode_number"),
                    });
            }
            episodes_by_season
        })
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    async fn merge_user_item_state_in_transaction(
        &self,
        transaction: &mut sqlx::Transaction<'_, Any>,
        source_item_id: &str,
        target_item_id: &str,
    ) -> Result<(), StorageError> {
        let max_function = self.scalar_max_function();
        let query = format!("INSERT INTO user_item_state (
                         user_id, item_id, position_ticks, is_played, is_favorite,
                         play_count, last_played_at, version
                     )
                     SELECT user_id, ?, position_ticks, is_played, is_favorite,
                            play_count, last_played_at, version
                     FROM user_item_state
                     WHERE item_id = ?
                     ON CONFLICT(user_id, item_id) DO UPDATE SET
                         position_ticks = {max_function}(user_item_state.position_ticks, excluded.position_ticks),
                         is_played = {max_function}(user_item_state.is_played, excluded.is_played),
                         is_favorite = {max_function}(user_item_state.is_favorite, excluded.is_favorite),
                         play_count = {max_function}(user_item_state.play_count, excluded.play_count),
                         last_played_at = CASE
                             WHEN user_item_state.last_played_at IS NULL THEN excluded.last_played_at
                             WHEN excluded.last_played_at IS NULL THEN user_item_state.last_played_at
                             WHEN excluded.last_played_at > user_item_state.last_played_at THEN excluded.last_played_at
                             ELSE user_item_state.last_played_at
                         END,
                         version = user_item_state.version + 1");
        self.query(sqlx::AssertSqlSafe(query))
            .bind(target_item_id)
            .bind(source_item_id)
            .execute(&mut **transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        self.query("DELETE FROM user_item_state WHERE item_id = ?")
            .bind(source_item_id)
            .execute(&mut **transaction)
            .await
            .map(|_| ())
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })
    }
}

fn validate_merge_root(
    primary: &MergeRootItem,
    roots: &[MergeRootItem],
) -> Result<(), StorageError> {
    if primary.removed_at.is_some() || primary.merged_into_item_id.is_some() {
        return Err(StorageError::Conflict("主条目已不可合并".to_owned()));
    }
    if !matches!(primary.item_type.as_str(), "MOVIE" | "SERIES") {
        return Err(StorageError::Conflict(
            "只能合并电影或剧集根条目".to_owned(),
        ));
    }
    if roots.iter().any(|root| {
        root.removed_at.is_some()
            || root.merged_into_item_id.is_some()
            || root.library_id != primary.library_id
            || root.item_type != primary.item_type
    }) {
        return Err(StorageError::Conflict(
            "待合并条目必须来自同一启用媒体库且类型相同".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::libraries::LibraryService, config::Config, library::LibraryKind};

    struct MergeTestItem<'a> {
        id: &'a str,
        item_type: &'a str,
        parent_id: Option<&'a str>,
        series_id: Option<&'a str>,
        season_number: Option<i64>,
        episode_number: Option<i64>,
    }

    async fn insert_merge_item(
        database: &Database,
        library_id: &str,
        item: MergeTestItem<'_>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO media_items (
                 id, library_id, item_type, parent_id, series_id,
                 season_number, episode_number, title, sort_title, identification_status
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'LOCAL_CONFIRMED')",
        )
        .bind(item.id)
        .bind(library_id)
        .bind(item.item_type)
        .bind(item.parent_id)
        .bind(item.series_id)
        .bind(item.season_number)
        .bind(item.episode_number)
        .bind(item.id)
        .bind(item.id.to_lowercase())
        .execute(database.pool())
        .await
        .map(|_| ())
    }

    #[tokio::test]
    async fn series_merge_batches_episode_reads_across_source_seasons()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let config = Config {
            http_addr: "127.0.0.1:8097".parse()?,
            config_dir: temp_dir.path().join("config"),
        };
        let database = Database::connect(&config).await?;
        let library = LibraryService::new(database.clone())
            .create_library("Shows", LibraryKind::Mixed, false)
            .await?;
        let target_series_id = "merge-target-series";
        let source_series_id = "merge-source-series";

        for (series_id, title) in [
            (target_series_id, "Target Series"),
            (source_series_id, "Source Series"),
        ] {
            sqlx::query(
                "INSERT INTO media_items (
                     id, library_id, item_type, title, sort_title, identification_status
                 ) VALUES (?, ?, 'SERIES', ?, ?, 'LOCAL_CONFIRMED')",
            )
            .bind(series_id)
            .bind(library.id.to_string())
            .bind(title)
            .bind(title.to_lowercase())
            .execute(database.pool())
            .await?;
        }

        for season_number in 1..=12 {
            let season_id = format!("merge-source-season-{season_number:02}");
            let season_title = format!("Season {season_number:02}");
            sqlx::query(
                "INSERT INTO media_items (
                     id, library_id, item_type, parent_id, series_id,
                     season_number, title, sort_title, identification_status
                 ) VALUES (?, ?, 'SEASON', ?, ?, ?, ?, ?, 'LOCAL_CONFIRMED')",
            )
            .bind(season_id)
            .bind(library.id.to_string())
            .bind(source_series_id)
            .bind(source_series_id)
            .bind(season_number)
            .bind(&season_title)
            .bind(season_title.to_lowercase())
            .execute(database.pool())
            .await?;
        }

        database.reset_query_count();
        database
            .merge_media_items(
                target_series_id,
                &[target_series_id.to_owned(), source_series_id.to_owned()],
            )
            .await?;

        assert_eq!(
            database.query_count(),
            21,
            "expected one batched episode read instead of one read per source season"
        );
        Ok(())
    }

    #[tokio::test]
    async fn series_merge_reuses_season_attached_by_an_earlier_source()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let config = Config {
            http_addr: "127.0.0.1:8097".parse()?,
            config_dir: temp_dir.path().join("config"),
        };
        let database = Database::connect(&config).await?;
        let library = LibraryService::new(database.clone())
            .create_library("Shows", LibraryKind::Mixed, false)
            .await?;
        let library_id = library.id.to_string();
        let target_series_id = "merge-target-series";
        let first_source_series_id = "merge-first-source-series";
        let second_source_series_id = "merge-second-source-series";
        let first_source_season_id = "merge-first-source-season";
        let second_source_season_id = "merge-second-source-season";
        let first_source_episode_id = "merge-first-source-episode";
        let second_source_episode_id = "merge-second-source-episode";

        for series_id in [
            target_series_id,
            first_source_series_id,
            second_source_series_id,
        ] {
            insert_merge_item(
                &database,
                &library_id,
                MergeTestItem {
                    id: series_id,
                    item_type: "SERIES",
                    parent_id: None,
                    series_id: None,
                    season_number: None,
                    episode_number: None,
                },
            )
            .await?;
        }
        insert_merge_item(
            &database,
            &library_id,
            MergeTestItem {
                id: first_source_season_id,
                item_type: "SEASON",
                parent_id: Some(first_source_series_id),
                series_id: Some(first_source_series_id),
                season_number: Some(1),
                episode_number: None,
            },
        )
        .await?;
        insert_merge_item(
            &database,
            &library_id,
            MergeTestItem {
                id: second_source_season_id,
                item_type: "SEASON",
                parent_id: Some(second_source_series_id),
                series_id: Some(second_source_series_id),
                season_number: Some(1),
                episode_number: None,
            },
        )
        .await?;
        insert_merge_item(
            &database,
            &library_id,
            MergeTestItem {
                id: first_source_episode_id,
                item_type: "EPISODE",
                parent_id: Some(first_source_season_id),
                series_id: Some(first_source_series_id),
                season_number: Some(1),
                episode_number: Some(1),
            },
        )
        .await?;
        insert_merge_item(
            &database,
            &library_id,
            MergeTestItem {
                id: second_source_episode_id,
                item_type: "EPISODE",
                parent_id: Some(second_source_season_id),
                series_id: Some(second_source_series_id),
                season_number: Some(1),
                episode_number: Some(1),
            },
        )
        .await?;

        database
            .merge_media_items(
                target_series_id,
                &[
                    target_series_id.to_owned(),
                    first_source_series_id.to_owned(),
                    second_source_series_id.to_owned(),
                ],
            )
            .await?;

        let first_season_parent = sqlx::query_scalar::<_, Option<String>>(
            "SELECT parent_id FROM media_items WHERE id = ?",
        )
        .bind(first_source_season_id)
        .fetch_one(database.pool())
        .await?;
        let second_season_merged_into = sqlx::query_scalar::<_, Option<String>>(
            "SELECT merged_into_item_id FROM media_items WHERE id = ?",
        )
        .bind(second_source_season_id)
        .fetch_one(database.pool())
        .await?;
        let second_episode_merged_into = sqlx::query_scalar::<_, Option<String>>(
            "SELECT merged_into_item_id FROM media_items WHERE id = ?",
        )
        .bind(second_source_episode_id)
        .fetch_one(database.pool())
        .await?;

        assert_eq!(first_season_parent.as_deref(), Some(target_series_id));
        assert_eq!(
            second_season_merged_into.as_deref(),
            Some(first_source_season_id)
        );
        assert_eq!(
            second_episode_merged_into.as_deref(),
            Some(first_source_episode_id)
        );
        Ok(())
    }
}
