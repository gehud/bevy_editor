/// Identifies a tab within a [`Node`](crate::Node).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct TabIndex(pub usize);

impl From<usize> for TabIndex {
    #[inline]
    fn from(index: usize) -> Self {
        TabIndex(index)
    }
}
