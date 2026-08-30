use ndarray::Array3;

pub type F32Image = Array3<f32>;

impl F32Image {
    pub fn new(width: usize, height: usize, channels: usize) -> Self {
        Self::zeros((channels, height, width))
    }

    pub fn width(&self) -> usize {
        self.shape()[2]
    }

    pub fn height(&self) -> usize {
        self.shape()[1]
    }

    pub fn channels(&self) -> usize {
        self.shape()[0]
    }
}
