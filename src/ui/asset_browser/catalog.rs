//! The browser's directory snapshot, indexed on Bevy's I/O task pool.

use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bevy::{
    asset::io::{ErasedAssetReader, embedded::EMBEDDED},
    prelude::*,
    tasks::{IoTaskPool, Task, futures::check_ready, futures_lite::StreamExt},
};

use crate::ProjectRoot;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Source {
    Project,
    Embedded,
}

impl Source {
    fn name(self) -> &'static str {
        match self {
            Self::Project => "Project Assets",
            Self::Embedded => "Embedded Assets (read-only)",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Folder {
    #[default]
    Sources,
    Asset(Source, PathBuf),
}

impl Folder {
    pub(super) fn name(&self) -> String {
        match self {
            Self::Sources => "Assets".into(),
            Self::Asset(source, path) => path.file_name().map_or_else(
                || source.name().into(),
                |name| name.to_string_lossy().into_owned(),
            ),
        }
    }

    pub(super) fn parent(&self) -> Option<Self> {
        match self {
            Self::Sources => None,
            Self::Asset(source, path) => Some(
                path.parent()
                    .map_or(Self::Sources, |parent| Self::Asset(*source, parent.into())),
            ),
        }
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) folder: Option<Folder>,
}

impl Entry {
    fn folder(folder: Folder) -> Self {
        Self {
            name: folder.name(),
            folder: Some(folder),
        }
    }

    fn file(name: &OsStr) -> Self {
        Self {
            name: name.to_string_lossy().into_owned(),
            folder: None,
        }
    }
}

#[derive(Default, Resource, PartialEq, Eq)]
pub(super) struct Catalog(pub BTreeMap<Folder, Result<Vec<Entry>, &'static str>>);

#[derive(Default)]
pub(super) struct Scan {
    project_assets: Option<PathBuf>,
    next: Option<Instant>,
    task: Option<Task<Catalog>>,
}

pub(super) fn refresh_catalog(
    project: Option<Res<ProjectRoot>>,
    server: Res<AssetServer>,
    mut scan: Local<Scan>,
    mut catalog: ResMut<Catalog>,
) {
    let project_assets = project.map(|root| root.0.join("assets"));
    if project_assets != scan.project_assets {
        scan.project_assets = project_assets;
        scan.task = None;
        scan.next = None;
    }
    if let Some(result) = scan.task.as_mut().and_then(check_ready) {
        catalog.set_if_neq(result);
        scan.task = None;
    }
    let now = Instant::now();
    if scan.task.is_none() && scan.next.is_none_or(|next| now >= next) {
        let project_assets = scan.project_assets.clone();
        let server = server.clone();
        scan.task = Some(IoTaskPool::get().spawn(build_catalog(project_assets, server)));
        scan.next = Some(now + Duration::from_secs(1));
    }
}

async fn build_catalog(project_assets: Option<PathBuf>, server: AssetServer) -> Catalog {
    let mut catalog = Catalog::default();
    let project = Folder::Asset(Source::Project, PathBuf::new());
    let embedded = Folder::Asset(Source::Embedded, PathBuf::new());
    catalog.0.insert(
        Folder::Sources,
        Ok(vec![
            Entry::folder(project.clone()),
            Entry::folder(embedded.clone()),
        ]),
    );
    if let Some(root) = project_assets {
        scan_project(&root, &mut catalog);
    } else {
        catalog.0.insert(project, Err("No project is open"));
    }
    if let Ok(source) = server.get_source(EMBEDDED) {
        scan_embedded(source.reader(), &mut catalog).await;
    } else {
        catalog
            .0
            .insert(embedded, Err("Embedded asset source unavailable"));
    }
    catalog
}

fn scan_project(root: &Path, catalog: &mut Catalog) {
    let mut pending = vec![PathBuf::new()];
    while let Some(path) = pending.pop() {
        let folder = Folder::Asset(Source::Project, path.clone());
        let children = match fs::read_dir(root.join(&path)) {
            Ok(children) => children,
            Err(error) => {
                let message = if error.kind() == std::io::ErrorKind::NotFound {
                    "Folder does not exist"
                } else {
                    "Cannot read folder"
                };
                catalog.0.insert(folder, Err(message));
                continue;
            }
        };
        let mut entries = Vec::new();
        for child in children.flatten() {
            let Ok(kind) = child.file_type() else {
                continue;
            };
            let name = child.file_name();
            if kind.is_dir() {
                let path = path.join(name);
                entries.push(Entry::folder(Folder::Asset(Source::Project, path.clone())));
                pending.push(path);
            } else if kind.is_file() {
                entries.push(Entry::file(&name));
            }
        }
        save_listing(folder, entries, catalog);
    }
}

async fn scan_embedded(reader: &dyn ErasedAssetReader, catalog: &mut Catalog) {
    let mut pending = vec![PathBuf::new()];
    while let Some(path) = pending.pop() {
        let Ok(mut children) = reader.read_directory(&path).await else {
            catalog.0.insert(
                Folder::Asset(Source::Embedded, path),
                Err("Cannot read embedded folder"),
            );
            continue;
        };
        let mut entries = Vec::new();
        while let Some(child) = children.next().await {
            let Some(name) = child.file_name() else {
                continue;
            };
            if reader.is_directory(&child).await == Ok(true) {
                entries.push(Entry::folder(Folder::Asset(
                    Source::Embedded,
                    child.clone(),
                )));
                pending.push(child);
            } else {
                entries.push(Entry::file(name));
            }
        }
        save_listing(Folder::Asset(Source::Embedded, path), entries, catalog);
    }
}

fn save_listing(folder: Folder, mut entries: Vec<Entry>, catalog: &mut Catalog) {
    entries.sort_by(|a, b| {
        a.folder
            .is_none()
            .cmp(&b.folder.is_none())
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.folder.cmp(&b.folder))
    });
    catalog.0.insert(folder, Ok(entries));
}
