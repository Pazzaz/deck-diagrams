use std::path::Path;

use resvg::{
    tiny_skia,
    usvg::{self, Tree},
};

pub fn svg_to_png(svg: &impl svg::Node, out: &Path) {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let opt = usvg::Options { fontdb: std::sync::Arc::new(fontdb), ..Default::default() };

    // Parse the SVG into a usvg tree
    let svg_data = svg.to_string();
    let tree = Tree::from_data(svg_data.as_bytes(), &opt).unwrap();

    // Create a buffer to render into
    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    pixmap.save_png(out).unwrap();
}
