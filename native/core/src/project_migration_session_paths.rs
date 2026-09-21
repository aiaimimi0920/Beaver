//! Resolve only declared Codex index paths against verified archive manifests.
use crate::{
    data_backup::Manifest, migration_bundle::Project, migration_import,
    project_migration_files::FileIndex,
};
use std::{collections::BTreeMap, path::Path};

type Check<T> = Result<T, &'static str>;

pub(crate) struct SessionPaths<'a> {
    application: &'a Manifest,
    projects: &'a [Project],
    index: &'a FileIndex<'a>,
}

impl<'a> SessionPaths<'a> {
    pub fn new(
        application: &'a Manifest,
        projects: &'a [Project],
        index: &'a FileIndex<'a>,
    ) -> Self {
        Self {
            application,
            projects,
            index,
        }
    }

    pub fn check(
        &self,
        database: &str,
        value: &str,
        rollout: bool,
        skipped: bool,
    ) -> Check<String> {
        let owner = self
            .index
            .owner(database)
            .map_err(|_| "FILE_OWNER_UNRESOLVED")?
            .ok_or("FILE_OWNER_UNRESOLVED")?;
        if let Some(relative) = migration_import::relative(&self.application.source, value)
            .map_err(|_| "INVALID_FILE_REFERENCE")?
        {
            if rollout {
                let home = database.rsplit_once('/').ok_or("FILE_OWNER_UNRESOLVED")?.0;
                let (prefix, _) = relative
                    .rsplit_once('/')
                    .ok_or("SESSION_ROLLOUT_OUTSIDE_HOME")?;
                if !prefix.eq_ignore_ascii_case(home)
                    && !prefix
                        .to_ascii_lowercase()
                        .starts_with(&format!("{}/", home.to_ascii_lowercase()))
                {
                    return Err("SESSION_ROLLOUT_OUTSIDE_HOME");
                }
            }
            let entry = self.index.entry(&relative).map_err(|reason| {
                if skipped && reason == "FILE_NOT_FOUND" {
                    "SESSION_SKIPPED_ROLLOUT_UNAVAILABLE"
                } else {
                    reason
                }
            })?;
            if rollout != entry.sha256.is_some() {
                return Err("FILE_TYPE_MISMATCH");
            }
            if self
                .index
                .owner(&relative)
                .map_err(|_| "FILE_OWNER_UNRESOLVED")?
                .as_deref()
                != Some(owner.as_str())
            {
                return Err("FILE_PROJECT_MISMATCH");
            }
            return Ok(format!("application/data/{}", entry.path));
        }
        if rollout {
            return Err("SESSION_ROLLOUT_OUTSIDE_HOME");
        }
        // Stored and canonical source paths may use different Windows prefixes.
        let mut matches = BTreeMap::new();
        for (position, project) in self.projects.iter().enumerate() {
            for root in [Path::new(&project.stored_path), project.source.as_path()] {
                if let Some(relative) =
                    migration_import::relative(root, value).map_err(|_| "INVALID_FILE_REFERENCE")?
                {
                    matches.insert((position, relative.to_ascii_lowercase()), relative);
                }
            }
        }
        if matches.len() > 1 {
            return Err("FILE_PATH_AMBIGUOUS");
        }
        let ((position, _), relative) = matches
            .into_iter()
            .next()
            .ok_or("SESSION_PATH_OUTSIDE_ARCHIVE")?;
        let project = &self.projects[position];
        if project.id != owner {
            return Err("FILE_PROJECT_MISMATCH");
        }
        let root = format!("projects/{}", project.id);
        if relative.is_empty() {
            return Ok(root);
        }
        let mut entries = project
            .entries
            .iter()
            .filter(|entry| entry.path.eq_ignore_ascii_case(&relative));
        let entry = entries.next().ok_or("FILE_NOT_FOUND")?;
        if entries.next().is_some() {
            return Err("FILE_PATH_AMBIGUOUS");
        }
        if entry.sha256.is_some() {
            return Err("FILE_TYPE_MISMATCH");
        }
        Ok(format!("{root}/{}", entry.path))
    }
}
