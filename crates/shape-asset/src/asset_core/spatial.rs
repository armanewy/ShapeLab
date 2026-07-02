
/// Asset-space transform stored as translation, XYZ Euler rotation, and scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3 {
    /// Local translation.
    pub translation: [f32; 3],
    /// XYZ Euler rotation in degrees.
    pub rotation_degrees: [f32; 3],
    /// Per-axis scale.
    pub scale: [f32; 3],
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_degrees: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform3 {
    /// Return the transform as a right-handed 4x4 matrix.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation_degrees[0].to_radians(),
            self.rotation_degrees[1].to_radians(),
            self.rotation_degrees[2].to_radians(),
        );
        Mat4::from_scale_rotation_translation(
            Vec3::from_array(self.scale),
            rotation,
            Vec3::from_array(self.translation),
        )
    }

    /// Transform a point by this transform.
    #[must_use]
    pub fn transform_point(&self, point: [f32; 3]) -> [f32; 3] {
        self.matrix()
            .transform_point3(Vec3::from_array(point))
            .to_array()
    }

    /// Transform a direction vector by this transform.
    #[must_use]
    pub fn transform_vector(&self, vector: [f32; 3]) -> [f32; 3] {
        self.matrix()
            .transform_vector3(Vec3::from_array(vector))
            .to_array()
    }
}

/// A local coordinate frame used for pivots and sockets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame3 {
    /// Frame origin.
    pub origin: [f32; 3],
    /// Local X axis.
    pub x_axis: [f32; 3],
    /// Local Y axis.
    pub y_axis: [f32; 3],
    /// Local Z axis.
    pub z_axis: [f32; 3],
}

impl Default for Frame3 {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0],
            z_axis: [0.0, 0.0, 1.0],
        }
    }
}

impl Frame3 {
    /// Return the frame transformed by an asset transform.
    #[must_use]
    pub fn transformed_by(&self, transform: &Transform3) -> Self {
        Self {
            origin: transform.transform_point(self.origin),
            x_axis: transform.transform_vector(self.x_axis),
            y_axis: transform.transform_vector(self.y_axis),
            z_axis: transform.transform_vector(self.z_axis),
        }
    }
}
