//! The browser's directory snapshot, indexed on Bevy's I/O task pool.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bevy::{
    asset::{
        AssetPath,
        io::{ErasedAssetReader, embedded::EMBEDDED},
    },
    prelude::*,
    tasks::{IoTaskPool, Task, futures::check_ready, futures_lite::StreamExt},
};

use crate::{ProjectRoot, project::PROJECT_ASSET_SOURCE};

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
    pub(super) kind: EntryKind,
}

#[derive(PartialEq, Eq)]
pub(super) enum EntryKind {
    Folder(Folder),
    File(AssetPath<'static>),
}

impl Entry {
    fn directory(folder: Folder) -> Self {
        Self {
            name: folder.name(),
            kind: EntryKind::Folder(folder),
        }
    }

    fn file(path: PathBuf, source: &'static str) -> Self {
        Self {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            kind: EntryKind::File(AssetPath::from_path_buf(path).with_source(source)),
        }
    }

    pub(super) fn folder(&self) -> Option<&Folder> {
        match &self.kind {
            EntryKind::Folder(folder) => Some(folder),
            EntryKind::File(_) => None,
        }
    }

    pub(super) fn asset_path(&self) -> Option<&AssetPath<'static>> {
        match &self.kind {
            EntryKind::Folder(_) => None,
            EntryKind::File(path) => Some(path),
        }
    }

    fn sort_key(&self) -> (bool, &str, &Path) {
        let path = match &self.kind {
            EntryKind::Folder(Folder::Asset(_, path)) => path,
            EntryKind::Folder(Folder::Sources) => Path::new(""),
            EntryKind::File(path) => path.path(),
        };
        (self.folder().is_none(), &self.name, path)
    }
}

#[derive(Default, Resource, PartialEq, Eq)]
pub(super) struct Catalog(pub BTreeMap<Folder, Result<Vec<Entry>, &'static str>>);

impl Catalog {
    pub(super) fn contains_file(&self, path: &AssetPath) -> bool {
        let source = match path.source().as_str() {
            Some(PROJECT_ASSET_SOURCE) => Source::Project,
            Some(EMBEDDED) => Source::Embedded,
            _ => return false,
        };
        let Some(parent) = path.path().parent() else {
            return false;
        };
        self.0
            .get(&Folder::Asset(source, parent.into()))
            .and_then(|listing| listing.as_ref().ok())
            .is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| matches!(&entry.kind, EntryKind::File(file) if file == path))
            })
    }
}

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
            Entry::directory(project.clone()),
            Entry::directory(embedded.clone()),
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
                entries.push(Entry::directory(Folder::Asset(
                    Source::Project,
                    path.clone(),
                )));
                pending.push(path);
            } else if kind.is_file() {
                entries.push(Entry::file(path.join(name), PROJECT_ASSET_SOURCE));
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
            if child.file_name().is_none() {
                continue;
            }
            if reader.is_directory(&child).await == Ok(true) {
                entries.push(Entry::directory(Folder::Asset(
                    Source::Embedded,
                    child.clone(),
                )));
                pending.push(child);
            } else {
                entries.push(Entry::file(child, EMBEDDED));
            }
        }
        save_listing(Folder::Asset(Source::Embedded, path), entries, catalog);
    }
}

fn save_listing(folder: Folder, mut entries: Vec<Entry>, catalog: &mut Catalog) {
    entries.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    catalog.0.insert(folder, Ok(entries));
}
