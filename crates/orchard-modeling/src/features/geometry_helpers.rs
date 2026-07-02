
fn normalize_profile(profile: &[[f32; 2]], feature: &'static str) -> FeatureResult<Vec<[f32; 2]>> {
    if profile.len() < 3 {
        return validation(feature, "profile requires at least three points");
    }
    let mut normalized = profile.to_vec();
    if points2_close(
        normalized[0],
        *normalized.last().expect("profile is not empty"),
    ) {
        normalized.pop();
    }
    if normalized.len() < 3 {
        return validation(feature, "profile requires at least three unique points");
    }
    for point in &normalized {
        if !point[0].is_finite() || !point[1].is_finite() {
            return validation(feature, "profile points must be finite");
        }
    }
    if signed_area(&normalized).abs() <= EPSILON {
        return validation(feature, "profile area must be non-zero");
    }
    Ok(normalized)
}

fn offset_points(
    points: &[[f32; 3]],
    offset: [f32; 3],
    feature: &'static str,
) -> FeatureResult<Vec<[f32; 3]>> {
    if offset.iter().any(|component| !component.is_finite()) {
        return validation(feature, "offset must be finite");
    }
    points
        .iter()
        .map(|point| {
            if point.iter().any(|component| !component.is_finite()) {
                validation(feature, "path points must be finite")
            } else {
                Ok([
                    point[0] + offset[0],
                    point[1] + offset[1],
                    point[2] + offset[2],
                ])
            }
        })
        .collect()
}

fn path_frames(
    points: &[[f32; 3]],
    up_hint: [f32; 3],
    roll_degrees: f32,
    closed: bool,
    feature: &'static str,
) -> FeatureResult<Vec<FrameBasis>> {
    let min_points = if closed { 3 } else { 2 };
    if points.len() < min_points {
        return validation(feature, "path has too few points");
    }
    let points = points
        .iter()
        .map(|point| Vec3::from_array(*point, feature))
        .collect::<FeatureResult<Vec<_>>>()?;
    let up = Vec3::from_array(up_hint, feature)?.normalized(feature, "up hint must be non-zero")?;
    let mut frames = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let tangent = path_tangent(&points, index, closed, feature)?;
        if tangent.dot(up).abs() > 0.999 {
            return validation(feature, "up hint must not be parallel to the path tangent");
        }
        let z = up
            .sub(tangent.scale(up.dot(tangent)))
            .normalized(feature, "path frame is degenerate")?;
        let x = tangent
            .cross(z)
            .normalized(feature, "path frame is degenerate")?;
        frames.push(
            FrameBasis {
                origin: points[index],
                x,
                y: tangent,
                z,
            }
            .rolled(roll_degrees),
        );
    }
    Ok(frames)
}

fn path_tangent(
    points: &[Vec3],
    index: usize,
    closed: bool,
    feature: &'static str,
) -> FeatureResult<Vec3> {
    let raw = if closed {
        let previous = points[(index + points.len() - 1) % points.len()];
        let next = points[(index + 1) % points.len()];
        next.sub(previous)
    } else if index == 0 {
        points[1].sub(points[0])
    } else if index + 1 == points.len() {
        points[index].sub(points[index - 1])
    } else {
        points[index + 1].sub(points[index - 1])
    };
    raw.normalized(feature, "path contains a zero-length tangent")
}

fn basis_from_frame(frame: &Frame3, feature: &'static str) -> FeatureResult<FrameBasis> {
    let origin = Vec3::from_array(frame.origin, feature)?;
    let x = Vec3::from_array(frame.x_axis, feature)?.normalized(feature, "frame x axis is zero")?;
    let y = Vec3::from_array(frame.y_axis, feature)?.normalized(feature, "frame y axis is zero")?;
    let z = Vec3::from_array(frame.z_axis, feature)?.normalized(feature, "frame z axis is zero")?;
    if x.dot(y).abs() > 1.0e-3 || y.dot(z).abs() > 1.0e-3 || z.dot(x).abs() > 1.0e-3 {
        return validation(feature, "frame axes must be orthogonal");
    }
    if x.cross(y).dot(z) < 0.99 {
        return validation(feature, "frame axes must be right-handed");
    }
    Ok(FrameBasis { origin, x, y, z })
}

#[derive(Debug, Copy, Clone)]
struct FrameBasis {
    origin: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl FrameBasis {
    fn transform_local(self, point: [f32; 3]) -> [f32; 3] {
        self.origin
            .add(self.x.scale(point[0]))
            .add(self.y.scale(point[1]))
            .add(self.z.scale(point[2]))
            .to_array()
    }

    fn rolled(self, roll_degrees: f32) -> Self {
        if roll_degrees.abs() <= EPSILON {
            return self;
        }
        let angle = roll_degrees.to_radians();
        Self {
            origin: self.origin,
            x: self.x.rotate_about(self.y, angle),
            y: self.y,
            z: self.z.rotate_about(self.y, angle),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct PathSample {
    position: Vec3,
    tangent: Vec3,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn from_array(value: [f32; 3], feature: &'static str) -> FeatureResult<Self> {
        if value.iter().any(|component| !component.is_finite()) {
            return validation(feature, "3D vectors must be finite");
        }
        Ok(Self {
            x: value[0],
            y: value[1],
            z: value[2],
        })
    }

    fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn scale(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self, feature: &'static str, message: &str) -> FeatureResult<Self> {
        let length = self.length();
        if !length.is_finite() || length <= EPSILON {
            return validation(feature, message);
        }
        Ok(self.scale(1.0 / length))
    }

    fn rotate_about(self, axis: Self, angle: f32) -> Self {
        let sin = angle.sin();
        let cos = angle.cos();
        self.scale(cos)
            .add(axis.cross(self).scale(sin))
            .add(axis.scale(axis.dot(self) * (1.0 - cos)))
    }
}

fn rounded_rect_points(
    half_x: f32,
    half_y: f32,
    radius: f32,
    corner_segments: u32,
) -> Vec<[f32; 2]> {
    if radius <= EPSILON {
        return vec![
            [half_x, half_y],
            [-half_x, half_y],
            [-half_x, -half_y],
            [half_x, -half_y],
        ];
    }
    let centers = [
        [half_x - radius, half_y - radius],
        [-half_x + radius, half_y - radius],
        [-half_x + radius, -half_y + radius],
        [half_x - radius, -half_y + radius],
    ];
    let starts = [0.0, FRAC_PI_2, PI, PI + FRAC_PI_2];
    let mut points = Vec::new();
    for (center, start) in centers.into_iter().zip(starts) {
        for index in 0..=corner_segments {
            let t = index as f32 / corner_segments as f32;
            let angle = start + t * FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            points.push([center[0] + radius * cos, center[1] + radius * sin]);
        }
    }
    points
}

fn perpendicular(axis: Vec3) -> Vec3 {
    let candidate = if axis.x.abs() < 0.9 {
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    };
    axis.cross(candidate)
        .normalized("fastener", "radial reference is degenerate")
        .expect("candidate should not be parallel")
}

fn positive(value: f32, feature: &'static str, label: &str) -> FeatureResult<f32> {
    if value.is_finite() && value > EPSILON {
        Ok(value)
    } else {
        validation(feature, &format!("{label} must be finite and positive"))
    }
}

fn non_negative(value: f32, feature: &'static str, label: &str) -> FeatureResult<f32> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        validation(feature, &format!("{label} must be finite and non-negative"))
    }
}

fn validation<T>(feature: &'static str, message: &str) -> FeatureResult<T> {
    Err(FeatureError::Validation {
        feature,
        message: message.to_owned(),
    })
}

fn points2_close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() <= EPSILON && (a[1] - b[1]).abs() <= EPSILON
}

fn signed_area(profile: &[[f32; 2]]) -> f32 {
    let mut area = 0.0;
    for index in 0..profile.len() {
        let current = profile[index];
        let next = profile[(index + 1) % profile.len()];
        area += current[0] * next[1] - next[0] * current[1];
    }
    area * 0.5
}

fn has_duplicate_indices(vertices: &[u32]) -> bool {
    let mut seen = BTreeSet::new();
    vertices.iter().any(|vertex| !seen.insert(*vertex))
}
