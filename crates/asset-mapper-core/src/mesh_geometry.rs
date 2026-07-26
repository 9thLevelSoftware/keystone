//! Lightweight triangle mesh sample used for socket proposal (no I/O).

use crate::schema::Vec3;

/// World-space mesh sample for connector detection.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshGeometry {
    pub positions: Vec<Vec3>,
    /// Optional triangle index list (groups of 3). When absent, positions are
    /// treated as an unindexed point cloud.
    pub indices: Option<Vec<u32>>,
}

impl MeshGeometry {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Triangle centroids (or subsampled positions when unindexed).
    pub fn sample_points(&self, max_samples: usize) -> Vec<Vec3> {
        if self.positions.is_empty() {
            return Vec::new();
        }

        let mut samples = Vec::new();
        if let Some(indices) = &self.indices {
            for tri in indices.chunks_exact(3) {
                let Ok(i0) = usize::try_from(tri[0]) else {
                    continue;
                };
                let Ok(i1) = usize::try_from(tri[1]) else {
                    continue;
                };
                let Ok(i2) = usize::try_from(tri[2]) else {
                    continue;
                };
                let (Some(a), Some(b), Some(c)) = (
                    self.positions.get(i0),
                    self.positions.get(i1),
                    self.positions.get(i2),
                ) else {
                    continue;
                };
                samples.push([
                    (a[0] + b[0] + c[0]) / 3.0,
                    (a[1] + b[1] + c[1]) / 3.0,
                    (a[2] + b[2] + c[2]) / 3.0,
                ]);
            }
        }

        if samples.is_empty() {
            samples = self.positions.clone();
        }

        if samples.len() > max_samples {
            let step = samples.len().div_ceil(max_samples).max(1);
            samples = samples.into_iter().step_by(step).collect();
        }
        samples
    }
}
