mod docs;
mod get;
mod index;
mod screenshot;

pub use docs::schema as schema_route;
pub use get::get_screenshot;
pub use index::index as index_route;
pub use screenshot::screenshot as screenshot_route;
