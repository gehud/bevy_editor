//! Crisp SVG icons for `bevy_ui`.
//!
//! Bevy UI draws images with bilinear filtering and no mipmaps, so a bitmap icon shown much
//! smaller than its pixel size looks jagged, and one shown larger looks blurry. This crate
//! instead renders each SVG at **exactly the physical size of the node that displays it**
//! (with [`resvg`]), so icons are pixel-sharp at any size and display scale.
//!
//! ```ignore
//! app.add_plugins(SvgPlugin);
//!
//! bsn! {
//!     Node { width: px(15), height: px(15) }
//!     SvgIcon { svg: "icons/eye.svg", mask: true, trim: false }
//!     ImageNode { color: Color::WHITE }
//! }
//! ```
//!
//! - Rendered images are cached by (SVG, pixel size, mask), so many icons of the same size share
//!   one texture, and a change of window scale factor simply adds new cache entries.
//! - With `mask: true` the SVG is rendered as a white silhouette (keeping its alpha). The color
//!   then comes from [`ImageNode::color`], so icons can be tinted, e.g. by `bevy_feathers`
//!   theming.

use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    ui::UiSystems,
};
use resvg::{tiny_skia, usvg};

pub struct SvgPlugin;

impl Plugin for SvgPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Svg>()
            .init_asset_loader::<SvgLoader>()
            .init_resource::<SvgRasterCache>()
            // After layout, so the node's final size for this frame is known, and before
            // rendering, so a newly spawned icon never shows without its image.
            .add_systems(PostUpdate, rasterize_svg_icons.after(UiSystems::Layout));
    }
}

/// A parsed SVG document.
#[derive(Asset, TypePath)]
pub struct Svg(pub usvg::Tree);

/// Displays an SVG in this UI node's [`ImageNode`], rendered at the node's exact size.
#[derive(Component, Clone, Debug, FromTemplate)]
#[require(ImageNode, RasterizedSvg)]
pub struct SvgIcon {
    pub svg: Handle<Svg>,
    /// Render a white silhouette, to be colored by [`ImageNode::color`].
    pub mask: bool,
    /// Render only the drawing's bounding box, ignoring empty margins in the SVG's canvas.
    pub trim: bool,
}

/// What is currently displayed, so unchanged icons are skipped.
#[derive(Component, Default)]
struct RasterizedSvg(Option<RasterKey>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RasterKey {
    svg: AssetId<Svg>,
    size: UVec2,
    mask: bool,
    trim: bool,
}

#[derive(Resource)]
struct SvgRasterCache {
    images: HashMap<RasterKey, Handle<Image>>,
    /// Shown while the SVG is still loading, instead of the default (opaque white) image.
    placeholder: Handle<Image>,
}

impl FromWorld for SvgRasterCache {
    fn from_world(world: &mut World) -> Self {
        let transparent = Image::new_fill(
            Extent3d::default(),
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        Self {
            images: HashMap::default(),
            placeholder: world.resource_mut::<Assets<Image>>().add(transparent),
        }
    }
}

fn rasterize_svg_icons(
    mut icons: Query<(&SvgIcon, &ComputedNode, &mut ImageNode, &mut RasterizedSvg)>,
    svgs: Res<Assets<Svg>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<SvgRasterCache>,
) {
    for (icon, node, mut image, mut rasterized) in &mut icons {
        let size = node.size().round().as_uvec2();
        if size.x == 0 || size.y == 0 {
            continue;
        }
        let Some(svg) = svgs.get(&icon.svg) else {
            if image.image != cache.placeholder {
                image.image = cache.placeholder.clone();
            }
            continue;
        };
        let key = RasterKey {
            svg: icon.svg.id(),
            size,
            mask: icon.mask,
            trim: icon.trim,
        };
        if rasterized.0 == Some(key) {
            continue;
        }
        let handle = match cache.images.get(&key) {
            Some(handle) => handle.clone(),
            None => {
                let handle = images.add(render(&svg.0, size, icon.mask, icon.trim));
                cache.images.insert(key, handle.clone());
                handle
            }
        };
        image.image = handle;
        rasterized.0 = Some(key);
    }
}

/// Renders `tree` (or, with `trim`, its drawing's bounding box) stretched to `size` pixels,
/// as an sRGB image with straight alpha.
fn render(tree: &usvg::Tree, size: UVec2, mask: bool, trim: bool) -> Image {
    let mut pixmap = tiny_skia::Pixmap::new(size.x, size.y).expect("non-zero size");
    let source = trim
        .then(|| tree.root().abs_stroke_bounding_box())
        .flatten()
        .map(|b| Rect::new(b.x(), b.y(), b.x() + b.width(), b.y() + b.height()))
        .unwrap_or(Rect::new(
            0.0,
            0.0,
            tree.size().width(),
            tree.size().height(),
        ));
    let scale = size.as_vec2() / source.size();
    let transform = tiny_skia::Transform::from_scale(scale.x, scale.y)
        .pre_translate(-source.min.x, -source.min.y);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    // tiny-skia stores premultiplied alpha; Bevy images expect straight alpha.
    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            if mask {
                [255, 255, 255, pixel.alpha()]
            } else {
                let c = pixel.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            }
        })
        .collect();

    Image::new(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Default, TypePath)]
struct SvgLoader;

#[derive(Debug)]
enum SvgLoaderError {
    Io(std::io::Error),
    Parse(usvg::Error),
}

impl std::fmt::Display for SvgLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvgLoaderError::Io(err) => write!(f, "could not read SVG: {err}"),
            SvgLoaderError::Parse(err) => write!(f, "could not parse SVG: {err}"),
        }
    }
}

impl std::error::Error for SvgLoaderError {}

impl AssetLoader for SvgLoader {
    type Asset = Svg;
    type Settings = ();
    type Error = SvgLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Svg, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(SvgLoaderError::Io)?;
        let tree = usvg::Tree::from_data(&bytes, &usvg::Options::default())
            .map_err(SvgLoaderError::Parse)?;
        Ok(Svg(tree))
    }

    fn extensions(&self) -> &[&str] {
        &["svg"]
    }
}
