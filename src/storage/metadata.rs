use super::*;

const MAX_ITEM_METADATA_COMPLETENESS_CAPABILITY_LENGTH: usize = 64;
const MAX_ITEM_METADATA_COMPLETENESS_FINGERPRINT_BYTES: usize = 256;
const MAX_ITEM_METADATA_COMPLETENESS_ERROR_BYTES: usize = 4096;
const MAX_ITEM_METADATA_COMPLETENESS_PAGE_SIZE: i64 = 100;

fn validate_item_metadata_completeness_key(
    item_id: &str,
    capability: &str,
    fingerprint: &[u8],
) -> Result<(), StorageError> {
    let capability_length = capability.trim().chars().count();
    if item_id.trim().is_empty()
        || !(1..=MAX_ITEM_METADATA_COMPLETENESS_CAPABILITY_LENGTH).contains(&capability_length)
        || fingerprint.is_empty()
        || fingerprint.len() > MAX_ITEM_METADATA_COMPLETENESS_FINGERPRINT_BYTES
    {
        return Err(StorageError::Conflict(
            "invalid item metadata completeness key or fingerprint".into(),
        ));
    }
    Ok(())
}

#[allow(dead_code)] // The local scan worker consumes these transitions in the next phase task.
impl Database {
    pub(crate) async fn prepare_item_metadata_completeness_check(
        &self,
        item_id: &str,
        capability: &str,
        input_fingerprint: &[u8],
    ) -> Result<bool, StorageError> {
        validate_item_metadata_completeness_key(item_id, capability, input_fingerprint)?;
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let changed = self
            .query_scalar::<String>(
                "INSERT INTO item_metadata_completeness (
                     item_id, capability, local_state, is_missing, input_fingerprint
                 ) VALUES (?, ?, 'PENDING', NULL, ?)
                 ON CONFLICT(item_id, capability) DO UPDATE SET
                     local_state = 'PENDING', is_missing = NULL,
                     input_fingerprint = excluded.input_fingerprint,
                     checked_at = NULL, retry_after = NULL, error = NULL,
                     updated_at = unixepoch()
                 WHERE item_metadata_completeness.input_fingerprint IS NULL
                    OR item_metadata_completeness.input_fingerprint <> excluded.input_fingerprint
                    OR item_metadata_completeness.local_state IN ('FAILED', 'CANCELLED')
                 RETURNING item_id",
            )
            .bind(item_id)
            .bind(capability.trim())
            .bind(input_fingerprint.to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(changed.is_some())
    }

    pub(crate) async fn claim_item_metadata_completeness_check(
        &self,
        item_id: &str,
        capability: &str,
        input_fingerprint: &[u8],
    ) -> Result<bool, StorageError> {
        validate_item_metadata_completeness_key(item_id, capability, input_fingerprint)?;
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let changed = self
            .query_scalar::<String>(
                "UPDATE item_metadata_completeness
                 SET local_state = 'RUNNING', is_missing = NULL, checked_at = NULL,
                     retry_after = NULL, error = NULL, updated_at = unixepoch()
                 WHERE item_id = ? AND capability = ? AND input_fingerprint = ?
                   AND local_state = 'PENDING'
                 RETURNING item_id",
            )
            .bind(item_id)
            .bind(capability.trim())
            .bind(input_fingerprint.to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(changed.is_some())
    }

    pub(crate) async fn finish_item_metadata_completeness_check(
        &self,
        item_id: &str,
        capability: &str,
        input_fingerprint: &[u8],
        is_missing: bool,
        checked_at: i64,
    ) -> Result<bool, StorageError> {
        validate_item_metadata_completeness_key(item_id, capability, input_fingerprint)?;
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let changed = self
            .query_scalar::<String>(
                "UPDATE item_metadata_completeness
                 SET local_state = 'READY', is_missing = ?, checked_at = ?, retry_after = NULL,
                     error = NULL, updated_at = unixepoch()
                 WHERE item_id = ? AND capability = ? AND input_fingerprint = ?
                   AND local_state = 'RUNNING'
                 RETURNING item_id",
            )
            .bind(database_flag(is_missing))
            .bind(checked_at)
            .bind(item_id)
            .bind(capability.trim())
            .bind(input_fingerprint.to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(changed.is_some())
    }

    pub(crate) async fn fail_item_metadata_completeness_check(
        &self,
        item_id: &str,
        capability: &str,
        input_fingerprint: &[u8],
        retry_after: Option<i64>,
        error: &str,
    ) -> Result<bool, StorageError> {
        validate_item_metadata_completeness_key(item_id, capability, input_fingerprint)?;
        if error.len() > MAX_ITEM_METADATA_COMPLETENESS_ERROR_BYTES {
            return Err(StorageError::Conflict(
                "item metadata completeness error exceeds the storage limit".into(),
            ));
        }
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let changed = self
            .query_scalar::<String>(
                "UPDATE item_metadata_completeness
                 SET local_state = 'FAILED', is_missing = NULL, checked_at = NULL,
                     retry_after = ?, error = ?, updated_at = unixepoch()
                 WHERE item_id = ? AND capability = ? AND input_fingerprint = ?
                   AND local_state = 'RUNNING'
                 RETURNING item_id",
            )
            .bind(retry_after)
            .bind(error)
            .bind(item_id)
            .bind(capability.trim())
            .bind(input_fingerprint.to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(changed.is_some())
    }

    pub(crate) async fn cancel_item_metadata_completeness_check(
        &self,
        item_id: &str,
        capability: &str,
        input_fingerprint: &[u8],
    ) -> Result<bool, StorageError> {
        validate_item_metadata_completeness_key(item_id, capability, input_fingerprint)?;
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let changed = self
            .query_scalar::<String>(
                "UPDATE item_metadata_completeness
                 SET local_state = 'CANCELLED', is_missing = NULL, checked_at = NULL,
                     retry_after = NULL, error = NULL, updated_at = unixepoch()
                 WHERE item_id = ? AND capability = ? AND input_fingerprint = ?
                   AND local_state IN ('PENDING', 'RUNNING')
                 RETURNING item_id",
            )
            .bind(item_id)
            .bind(capability.trim())
            .bind(input_fingerprint.to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(changed.is_some())
    }

    /// Call once during startup, before local metadata workers begin claiming checks.
    pub(crate) async fn requeue_interrupted_item_metadata_completeness_checks(
        &self,
    ) -> Result<u64, StorageError> {
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let result = self
            .query(
                "UPDATE item_metadata_completeness
                 SET local_state = 'PENDING', is_missing = NULL, checked_at = NULL,
                     retry_after = NULL, error = NULL, updated_at = unixepoch()
                 WHERE local_state = 'RUNNING'",
            )
            .execute(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(result.rows_affected())
    }

    pub(crate) async fn find_item_metadata_completeness(
        &self,
        item_id: &str,
        capability: &str,
    ) -> Result<Option<StoredItemMetadataCompleteness>, StorageError> {
        if item_id.trim().is_empty()
            || capability.trim().is_empty()
            || capability.trim().chars().count() > MAX_ITEM_METADATA_COMPLETENESS_CAPABILITY_LENGTH
        {
            return Err(StorageError::Conflict(
                "invalid item metadata completeness key".into(),
            ));
        }
        self.query(
            "SELECT item_id, capability, local_state, is_missing, input_fingerprint,
                    checked_at, retry_after, error, updated_at
             FROM item_metadata_completeness WHERE item_id = ? AND capability = ?",
        )
        .bind(item_id)
        .bind(capability.trim())
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(stored_item_metadata_completeness))
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn list_confirmed_missing_metadata(
        &self,
        capability: &str,
        after_item_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<StoredItemMetadataCompleteness>, StorageError> {
        let capability_length = capability.trim().chars().count();
        let limit = i64::try_from(limit).map_err(|_| {
            StorageError::Conflict("invalid metadata completeness page size".into())
        })?;
        if !(1..=MAX_ITEM_METADATA_COMPLETENESS_CAPABILITY_LENGTH).contains(&capability_length)
            || !(1..=MAX_ITEM_METADATA_COMPLETENESS_PAGE_SIZE).contains(&limit)
            || after_item_id.is_some_and(|item_id| item_id.trim().is_empty())
        {
            return Err(StorageError::Conflict(
                "invalid metadata completeness filter or page size".into(),
            ));
        }
        let rows = if let Some(after_item_id) = after_item_id {
            self.query(
                "SELECT completeness.item_id, completeness.capability, completeness.local_state,
                        completeness.is_missing, completeness.input_fingerprint,
                        completeness.checked_at, completeness.retry_after, completeness.error,
                        completeness.updated_at
                 FROM item_metadata_completeness completeness
                 JOIN media_items ON media_items.id = completeness.item_id
                 WHERE completeness.capability = ? AND completeness.local_state = 'READY'
                   AND completeness.is_missing = 1
                   AND media_items.removed_at IS NULL
                   AND completeness.item_id > ?
                 ORDER BY completeness.item_id LIMIT ?",
            )
            .bind(capability.trim())
            .bind(after_item_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            self.query(
                "SELECT completeness.item_id, completeness.capability, completeness.local_state,
                        completeness.is_missing, completeness.input_fingerprint,
                        completeness.checked_at, completeness.retry_after, completeness.error,
                        completeness.updated_at
                 FROM item_metadata_completeness completeness
                 JOIN media_items ON media_items.id = completeness.item_id
                 WHERE completeness.capability = ? AND completeness.local_state = 'READY'
                   AND completeness.is_missing = 1
                   AND media_items.removed_at IS NULL
                 ORDER BY completeness.item_id LIMIT ?",
            )
            .bind(capability.trim())
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        Ok(rows
            .into_iter()
            .map(stored_item_metadata_completeness)
            .collect())
    }
}

impl Database {
    pub(crate) async fn count_pending_metadata_candidates(&self) -> Result<i64, StorageError> {
        self.query_scalar("SELECT COUNT(*) FROM metadata_candidates WHERE status = 'PENDING'")
            .fetch_one(&self.pool)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })
    }

    pub(crate) async fn list_pending_metadata_item_ids(
        &self,
        item_ids: &[String],
    ) -> Result<HashSet<String>, StorageError> {
        let mut pending = HashSet::new();
        for chunk in item_ids.chunks(500) {
            if chunk.is_empty() {
                continue;
            }
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT DISTINCT item_id FROM metadata_candidates
                 WHERE status = 'PENDING' AND item_id IN ({placeholders})"
            );
            let mut statement = self.query(sqlx::AssertSqlSafe(query));
            for item_id in chunk {
                statement = statement.bind(item_id);
            }
            let rows =
                statement
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|source| StorageError::Sqlx {
                        path: self.path.clone(),
                        source,
                    })?;
            pending.extend(rows.into_iter().map(|row| row.get("item_id")));
        }
        Ok(pending)
    }

    pub(crate) async fn insert_metadata_candidate(
        &self,
        candidate: NewMetadataCandidate<'_>,
    ) -> Result<(), StorageError> {
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        self.query(
            "INSERT INTO metadata_candidates (
                id, item_id, provider, provider_id, candidate_json, score, status, expires_at
            ) VALUES (?, ?, ?, ?, ?, ?, 'PENDING', ?)
            ON CONFLICT (item_id, provider, provider_id) WHERE status = 'PENDING'
            DO UPDATE SET
                candidate_json = CASE
                    WHEN excluded.score >= metadata_candidates.score
                    THEN excluded.candidate_json
                    ELSE metadata_candidates.candidate_json
                END,
                score = CASE
                    WHEN excluded.score >= metadata_candidates.score
                    THEN excluded.score
                    ELSE metadata_candidates.score
                END,
                expires_at = CASE
                    WHEN excluded.score >= metadata_candidates.score
                    THEN excluded.expires_at
                    ELSE metadata_candidates.expires_at
                END,
                updated_at = unixepoch()",
        )
        .bind(candidate.id)
        .bind(candidate.item_id)
        .bind(candidate.provider)
        .bind(candidate.provider_id)
        .bind(candidate.candidate_json)
        .bind(candidate.score)
        .bind(candidate.expires_at)
        .execute(&mut *transaction)
        .await
        .map(|_| ())
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })
    }

    pub(crate) async fn update_pending_metadata_candidate_json(
        &self,
        item_id: &str,
        candidate_id: &str,
        candidate_json: &str,
    ) -> Result<bool, StorageError> {
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let result = self
            .query(
                "UPDATE metadata_candidates
                 SET candidate_json = ?, updated_at = unixepoch()
                 WHERE id = ? AND item_id = ? AND status = 'PENDING'",
            )
            .bind(candidate_json)
            .bind(candidate_id)
            .bind(item_id)
            .execute(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn list_metadata_capability_attempts(
        &self,
        item_id: &str,
    ) -> Result<Vec<StoredMetadataCapabilityAttempt>, StorageError> {
        self.query(
            "SELECT provider, provider_id, capability, status, next_retry_at
             FROM metadata_capability_attempts
             WHERE item_id = ?",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| StoredMetadataCapabilityAttempt {
                    provider: row.get("provider"),
                    provider_id: row.get("provider_id"),
                    capability: row.get("capability"),
                    status: row.get("status"),
                    next_retry_at: row.get("next_retry_at"),
                })
                .collect()
        })
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn list_metadata_image_attempts(
        &self,
        item_id: &str,
    ) -> Result<Vec<StoredMetadataImageAttempt>, StorageError> {
        self.query(
            "SELECT image_type, candidate_key, status
             FROM metadata_image_attempts
             WHERE item_id = ?",
        )
        .bind(item_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| StoredMetadataImageAttempt {
                    image_type: row.get("image_type"),
                    candidate_key: row.get("candidate_key"),
                    status: row.get("status"),
                })
                .collect()
        })
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn record_metadata_capability_results(
        &self,
        item_id: &str,
        provider: &str,
        provider_id: &str,
        results: &[MetadataCapabilityResult<'_>],
        now: i64,
    ) -> Result<(), StorageError> {
        if results.is_empty() {
            return Ok(());
        }
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        for result in results {
            let status = if result.has_data {
                "AVAILABLE"
            } else {
                "UNAVAILABLE"
            };
            let error_code = (!result.has_data).then_some("NO_DATA");
            self.query(
                "INSERT INTO metadata_capability_attempts (
                    item_id, provider, provider_id, capability, status, attempt_count,
                    last_attempt_at, next_retry_at, error_code, updated_at
                ) VALUES (?, ?, ?, ?, ?, 1, ?, NULL, ?, ?)
                ON CONFLICT(item_id, provider, provider_id, capability) DO UPDATE SET
                    status = excluded.status,
                    attempt_count = 1,
                    last_attempt_at = excluded.last_attempt_at,
                    next_retry_at = NULL,
                    error_code = excluded.error_code,
                    updated_at = excluded.updated_at",
            )
            .bind(item_id)
            .bind(provider)
            .bind(provider_id)
            .bind(result.capability)
            .bind(status)
            .bind(now)
            .bind(error_code)
            .bind(now)
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
            })
    }

    pub(crate) async fn record_metadata_capability_failure(
        &self,
        item_id: &str,
        provider: &str,
        provider_id: &str,
        capability: &str,
        now: i64,
    ) -> Result<(), StorageError> {
        let _write_guard = self.acquire_metadata_write_lock().await;
        let mut transaction = self.begin_metadata_write_transaction().await?;
        let previous_attempt_count = self
            .query_scalar::<i64>(
                "SELECT attempt_count
                 FROM metadata_capability_attempts
                 WHERE item_id = ? AND provider = ? AND provider_id = ? AND capability = ?",
            )
            .bind(item_id)
            .bind(provider)
            .bind(provider_id)
            .bind(capability)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?
            .unwrap_or_default();
        let exponent = previous_attempt_count.clamp(0, 5) as u32;
        let delay = 300_i64
            .saturating_mul(1_i64.checked_shl(exponent).unwrap_or(i64::MAX))
            .min(86_400);
        let next_retry_at = now.saturating_add(delay);
        self.query(
            "INSERT INTO metadata_capability_attempts (
                item_id, provider, provider_id, capability, status, attempt_count,
                last_attempt_at, next_retry_at, error_code, updated_at
            ) VALUES (?, ?, ?, ?, 'FAILED', 1, ?, ?, 'TRANSIENT_FAILURE', ?)
            ON CONFLICT(item_id, provider, provider_id, capability) DO UPDATE SET
                status = 'FAILED',
                attempt_count = metadata_capability_attempts.attempt_count + 1,
                last_attempt_at = excluded.last_attempt_at,
                next_retry_at = excluded.next_retry_at,
                error_code = excluded.error_code,
                updated_at = excluded.updated_at",
        )
        .bind(item_id)
        .bind(provider)
        .bind(provider_id)
        .bind(capability)
        .bind(now)
        .bind(next_retry_at)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })
    }

    pub(crate) async fn list_pending_metadata_candidates(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<StoredMetadataCandidate>, StorageError> {
        self.query(
            "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                    mc.candidate_json, mc.score, mc.status, mc.expires_at,
                    mi.title AS item_title
             FROM metadata_candidates mc
             JOIN media_items mi ON mi.id = mc.item_id
             WHERE mc.status = 'PENDING' AND mi.removed_at IS NULL
             ORDER BY mc.created_at, mc.id
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(stored_metadata_candidate).collect())
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn count_pending_metadata_candidates_for_item(
        &self,
        item_id: &str,
        search: Option<&str>,
    ) -> Result<i64, StorageError> {
        let count = if let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) {
            let pattern = format!("%{search}%");
            self.query_scalar::<i64>(
                "SELECT COUNT(*) FROM metadata_candidates
                 WHERE item_id = ? AND status = 'PENDING'
                   AND (provider_id LIKE ? OR candidate_json LIKE ?)",
            )
            .bind(item_id)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await
        } else {
            self.query_scalar::<i64>(
                "SELECT COUNT(*) FROM metadata_candidates
                 WHERE item_id = ? AND status = 'PENDING'",
            )
            .bind(item_id)
            .fetch_one(&self.pool)
            .await
        };
        count.map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn list_pending_metadata_candidates_for_item(
        &self,
        item_id: &str,
        search: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<StoredMetadataCandidate>, StorageError> {
        let rows = if let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) {
            let pattern = format!("%{search}%");
            self.query(
                "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                        mc.candidate_json, mc.score, mc.status, mc.expires_at,
                        mi.title AS item_title
                 FROM metadata_candidates mc
                 JOIN media_items mi ON mi.id = mc.item_id
                 WHERE mc.item_id = ? AND mc.status = 'PENDING' AND mi.removed_at IS NULL
                   AND (mc.provider_id LIKE ? OR mc.candidate_json LIKE ?)
                 ORDER BY mc.created_at, mc.id LIMIT ? OFFSET ?",
            )
            .bind(item_id)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            self.query(
                "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                        mc.candidate_json, mc.score, mc.status, mc.expires_at,
                        mi.title AS item_title
                 FROM metadata_candidates mc
                 JOIN media_items mi ON mi.id = mc.item_id
                 WHERE mc.item_id = ? AND mc.status = 'PENDING' AND mi.removed_at IS NULL
                 ORDER BY mc.created_at, mc.id LIMIT ? OFFSET ?",
            )
            .bind(item_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        };
        rows.map(|rows| rows.into_iter().map(stored_metadata_candidate).collect())
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })
    }

    pub(crate) async fn find_metadata_candidate(
        &self,
        item_id: &str,
        candidate_id: &str,
    ) -> Result<Option<StoredMetadataCandidate>, StorageError> {
        self.query(
            "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                    mc.candidate_json, mc.score, mc.status, mc.expires_at,
                    mi.title AS item_title
             FROM metadata_candidates mc
             JOIN media_items mi ON mi.id = mc.item_id
             WHERE mc.id = ? AND mc.item_id = ?
               AND mi.removed_at IS NULL
             LIMIT 1",
        )
        .bind(candidate_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(stored_metadata_candidate))
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn find_best_pending_metadata_candidate(
        &self,
        item_id: &str,
    ) -> Result<Option<StoredMetadataCandidate>, StorageError> {
        self.query(
            "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                    mc.candidate_json, mc.score, mc.status, mc.expires_at,
                    mi.title AS item_title
             FROM metadata_candidates mc
             JOIN media_items mi ON mi.id = mc.item_id
             WHERE mc.item_id = ? AND mc.status = 'PENDING'
               AND mi.removed_at IS NULL
             ORDER BY mc.score DESC, mc.created_at, mc.id
             LIMIT 1",
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(stored_metadata_candidate))
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn list_unexpired_pending_metadata_candidates_for_item(
        &self,
        item_id: &str,
        limit: i64,
    ) -> Result<Vec<StoredMetadataCandidate>, StorageError> {
        self.query(
            "SELECT mc.id, mc.item_id, mc.provider, mc.provider_id,
                    mc.candidate_json, mc.score, mc.status, mc.expires_at,
                    mi.title AS item_title
             FROM metadata_candidates mc
             JOIN media_items mi ON mi.id = mc.item_id
             WHERE mc.item_id = ? AND mc.status = 'PENDING'
               AND mi.removed_at IS NULL
               AND (mc.expires_at IS NULL OR mc.expires_at > unixepoch())
             ORDER BY mc.score DESC, mc.created_at, mc.id
             LIMIT ?",
        )
        .bind(item_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(stored_metadata_candidate).collect())
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })
    }

    pub(crate) async fn select_metadata_candidate(
        &self,
        update: SelectedMetadataUpdate<'_>,
    ) -> Result<bool, StorageError> {
        let sort_title = update.title.to_lowercase();
        let _write_guard = self.acquire_metadata_write_lock().await;
        // SQLite WAL can reject a deferred read-to-write upgrade with
        // SQLITE_BUSY_SNAPSHOT; reserve the single writer before this short
        // metadata transaction performs its updates.
        let mut transaction = self.begin_metadata_write_transaction().await?;
        self.query(
            "UPDATE media_items
             SET title = ?, sort_title = ?, original_title = ?, overview = ?, production_year = ?,
                 premiere_date = COALESCE(?, premiere_date),
                 last_air_date = COALESCE(?, last_air_date),
                 status = COALESCE(?, status),
                 original_language = COALESCE(?, original_language),
                 rating = CASE WHEN ? = 1 THEN ? ELSE rating END,
                 rating_source = CASE WHEN ? IS NULL THEN rating_source ELSE ? END,
                 provider_ids_json = ?,
                 metadata_scraper_id = CASE WHEN ? IS NULL THEN metadata_scraper_id ELSE ? END,
                 identification_status = CASE WHEN ? = 1 THEN 'PENDING' ELSE 'ONLINE_CONFIRMED' END,
                 metadata_fingerprint = ?, metadata_provenance_json = ?, locked_fields_json = ?,
                 poster_fallback_required = ?, updated_at = unixepoch()
             WHERE id = ? AND removed_at IS NULL",
        )
        .bind(update.title)
        .bind(sort_title)
        .bind(update.original_title)
        .bind(update.overview)
        .bind(update.production_year)
        .bind(update.premiere_date)
        .bind(update.last_air_date)
        .bind(update.status)
        .bind(update.original_language)
        .bind(database_flag(update.rating.is_some()))
        .bind(update.rating.unwrap_or_default())
        .bind(update.rating_source)
        .bind(update.rating_source)
        .bind(update.provider_ids_json)
        .bind(update.metadata_scraper_id)
        .bind(update.metadata_scraper_id)
        .bind(database_flag(update.keep_pending))
        .bind(update.metadata_fingerprint)
        .bind(update.provenance_json)
        .bind(update.locked_fields_json)
        .bind(database_flag(update.poster_fallback_required))
        .bind(update.item_id)
        .execute(&mut *transaction)
        .await
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        let selected = self
            .query(
                "UPDATE metadata_candidates
             SET status = CASE WHEN ? = 1 THEN 'PENDING' ELSE 'SELECTED' END,
                 updated_at = unixepoch()
             WHERE id = ? AND item_id = ? AND status = 'PENDING'",
            )
            .bind(database_flag(update.keep_pending))
            .bind(update.candidate_id)
            .bind(update.item_id)
            .execute(&mut *transaction)
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        if selected.rows_affected() != 1 {
            transaction
                .rollback()
                .await
                .map_err(|source| StorageError::Sqlx {
                    path: self.path.clone(),
                    source,
                })?;
            return Ok(false);
        }
        self.query(
            "UPDATE metadata_candidates
             SET status = 'REJECTED', updated_at = unixepoch()
             WHERE item_id = ? AND status = 'PENDING' AND id <> ?",
        )
        .bind(update.item_id)
        .bind(update.candidate_id)
        .execute(&mut *transaction)
        .await
        .map_err(|source| StorageError::Sqlx {
            path: self.path.clone(),
            source,
        })?;
        transaction
            .commit()
            .await
            .map_err(|source| StorageError::Sqlx {
                path: self.path.clone(),
                source,
            })?;
        Ok(true)
    }
}
