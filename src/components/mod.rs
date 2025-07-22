pub mod body;
#[cfg(feature = "trails")]
pub mod trail;

pub use body::BodyBundle;
#[cfg(feature = "trails")]
pub use trail::Trail;
