//! Read-only inspection of the root asset and labels emitted by Bevy's asset loaders.

use std::path::{Path, PathBuf};

use bevy::{
    asset::{
        AssetLoader, AssetPath, LoadContext,
        io::{AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader, VecReader},
    },
    prelude::*,
};

const INSPECTION_SOURCE: &str = "editor_inspection";

pub(crate) fn register_asset_inspection_source(app: &mut App) {
    app.register_asset_source(
        INSPECTION_SOURCE,
        AssetSourceBuilder::new(|| Box::new(InspectionReader)),
    );
}

/// Virtual requests have no file of their own. The requested file is loaded by Bevy inside
/// `DetailsLoader`, where the labeled assets can be read before the loader's output is dropped.
struct InspectionReader;

impl AssetReader for InspectionReader {
    async fn read(&self, _path: &Path) -> Result<impl Reader, AssetReaderError> {
        Ok(VecReader::new(Vec::new()))
    }

    async fn read_meta(&self, path: &Path) -> Result<impl Reader, AssetReaderError> {
        Err::<VecReader, _>(AssetReaderError::NotFound(path.into()))
    }

    async fn read_directory(&self, path: &Path) -> Result<Box<PathStream>, AssetReaderError> {
        Err(AssetReaderError::NotFound(path.into()))
    }

    async fn is_directory(&self, _path: &Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }
}

#[derive(Asset, TypePath)]
pub(super) struct AssetDetails(pub(super) Result<FileLabels, String>);

pub(super) struct FileLabels {
    pub(super) root_type: &'static str,
    pub(super) labels: Vec<(String, &'static str)>,
}

#[derive(Default, TypePath)]
pub(super) struct DetailsLoader;

impl AssetLoader for DetailsLoader {
    type Asset = AssetDetails;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        context: &mut LoadContext<'_>,
    ) -> Result<AssetDetails, BevyError> {
        let mut parts = context.path().path().components();
        let source = parts
            .next()
            .and_then(|part| part.as_os_str().to_str())
            .ok_or("Asset inspection request has no source")?
            .to_owned();
        let path = AssetPath::from_path_buf(parts.as_path().into()).with_source(source);
        let loaded = match context.load_builder().load_untyped_value(path).await {
            Ok(loaded) => loaded,
            Err(error) => return Ok(AssetDetails(Err(error.to_string()))),
        };
        let mut labels = loaded
            .iter_labels()
            .map(|label| {
                let asset = loaded
                    .get_labeled(label)
                    .expect("label came from the loader");
                (label.to_owned(), short_type(asset.asset_type_name()))
            })
            .collect::<Vec<_>>();
        labels.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        Ok(AssetDetails(Ok(FileLabels {
            root_type: short_type(loaded.asset_type_name()),
            labels,
        })))
    }
}

pub(super) fn load_details(server: &AssetServer, path: &AssetPath) -> Option<Handle<AssetDetails>> {
    let source = path.source().as_str()?;
    let request = AssetPath::from_path_buf(PathBuf::from(source).join(path.path()))
        .with_source(INSPECTION_SOURCE);
    Some(server.load(request))
}

fn short_type(name: &str) -> &str {
    let base = name.split('<').next().unwrap_or(name);
    let start = base.rfind("::").map_or(0, |separator| separator + 2);
    &name[start..]
}
