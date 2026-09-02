use std::ops::{Deref, DerefMut, Sub};

use ndarray::Array3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 3-channel float image stored as `(channels, height, width)`.
///
/// This is a newtype wrapper around [`Array3<f32>`] so the crate can define
/// inherent constructor / accessor methods without violating Rust's orphan
/// rule (which forbids `impl` on a foreign type, even via a type alias).
///
/// All `Array3` methods remain accessible through `Deref`/`DerefMut`, so
/// call sites that index with `img[(c, y, x)]`, call `.iter()`, `.sum()`,
/// `.len()`, `.mean()`, etc. work unchanged. Pixel-wise arithmetic
/// (`a - b`) is supported via the `Sub` impls below.
#[derive(Debug, Clone)]
pub struct F32Image(Array3<f32>);

impl F32Image {
    /// Create a zero-filled image with `channels × height × width` elements.
    /// The underlying `Array3` stores shape as `(channels, height, width)`.
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self(Array3::zeros((channels, height, width)))
    }

    pub fn width(&self) -> usize {
        self.0.shape()[2]
    }

    pub fn height(&self) -> usize {
        self.0.shape()[1]
    }

    pub fn channels(&self) -> usize {
        self.0.shape()[0]
    }
}

impl Deref for F32Image {
    type Target = Array3<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for F32Image {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Array3<f32>> for F32Image {
    fn from(arr: Array3<f32>) -> Self {
        Self(arr)
    }
}

impl From<F32Image> for Array3<f32> {
    fn from(img: F32Image) -> Self {
        img.0
    }
}

// Pixel-wise subtraction. Without these, `dark - bias` and `&result - &dark`
// (which the rest of the crate relies on) fail to compile because `Sub`
// is only implemented for `Array3<f32>`, not for our newtype wrapper.

impl Sub for &F32Image {
    type Output = F32Image;

    fn sub(self, rhs: &F32Image) -> F32Image {
        F32Image(&self.0 - &rhs.0)
    }
}

impl Sub for F32Image {
    type Output = F32Image;

    fn sub(self, rhs: F32Image) -> F32Image {
        F32Image(self.0 - rhs.0)
    }
}

// Forward serde through the wrapped Array3. The derive macro can't reach
// into a private field through the orphan rule, so we hand-roll.

impl Serialize for F32Image {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for F32Image {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Array3::<f32>::deserialize(deserializer).map(F32Image)
    }
}
