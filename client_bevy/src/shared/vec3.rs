#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

pub fn length(v: Vec3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

pub fn normalize(v: Vec3) -> Vec3 {
    let len = length(v);
    if len < 1e-10 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(v.x / len, v.y / len, v.z / len)
    }
}

pub fn build_tangent_basis(u: Vec3) -> (Vec3, Vec3) {
    let reference = if dot(u, Vec3::new(0.0, 1.0, 0.0)).abs() < 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    let e1 = normalize(cross(reference, u));
    let e2 = cross(u, e1);
    (e1, e2)
}

pub fn rotate_normalize_in_place(pos: &mut Vec3, axis: Vec3, angle: f64) {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let one_minus_cos = 1.0 - cos_a;

    let cx = axis.y * pos.z - axis.z * pos.y;
    let cy = axis.z * pos.x - axis.x * pos.z;
    let cz = axis.x * pos.y - axis.y * pos.x;

    let d = axis.x * pos.x + axis.y * pos.y + axis.z * pos.z;

    let rx = pos.x * cos_a + cx * sin_a + axis.x * d * one_minus_cos;
    let ry = pos.y * cos_a + cy * sin_a + axis.y * d * one_minus_cos;
    let rz = pos.z * cos_a + cz * sin_a + axis.z * d * one_minus_cos;

    let len = (rx * rx + ry * ry + rz * rz).sqrt();
    if len < 1e-10 {
        pos.x = 1.0;
        pos.y = 0.0;
        pos.z = 0.0;
    } else {
        let inv = 1.0 / len;
        pos.x = rx * inv;
        pos.y = ry * inv;
        pos.z = rz * inv;
    }
}
