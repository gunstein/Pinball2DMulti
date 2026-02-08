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

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn assert_vec3_close(a: Vec3, b: Vec3, eps: f64) {
        assert!((a.x - b.x).abs() < eps, "x: {} vs {}", a.x, b.x);
        assert!((a.y - b.y).abs() < eps, "y: {} vs {}", a.y, b.y);
        assert!((a.z - b.z).abs() < eps, "z: {} vs {}", a.z, b.z);
    }

    mod basic_ops {
        use super::*;

        #[test]
        fn dot_orthogonal_is_zero() {
            assert_eq!(dot(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)), 0.0);
        }

        #[test]
        fn dot_parallel_is_product_of_lengths() {
            assert_eq!(dot(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 0.0, 0.0)), 6.0);
        }

        #[test]
        fn dot_antiparallel_is_negative() {
            assert_eq!(
                dot(Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)),
                -1.0
            );
        }

        #[test]
        fn cross_x_y_is_z() {
            assert_vec3_close(
                cross(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
                Vec3::new(0.0, 0.0, 1.0),
                1e-10,
            );
        }

        #[test]
        fn cross_parallel_is_zero() {
            assert_vec3_close(
                cross(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)),
                Vec3::new(0.0, 0.0, 0.0),
                1e-10,
            );
        }

        #[test]
        fn length_of_unit_vectors() {
            assert_eq!(length(Vec3::new(1.0, 0.0, 0.0)), 1.0);
            assert_eq!(length(Vec3::new(0.0, 1.0, 0.0)), 1.0);
            assert_eq!(length(Vec3::new(0.0, 0.0, 1.0)), 1.0);
        }

        #[test]
        fn length_3_4_0_is_5() {
            assert_eq!(length(Vec3::new(3.0, 4.0, 0.0)), 5.0);
        }

        #[test]
        fn normalize_returns_unit_vector() {
            let v = normalize(Vec3::new(3.0, 4.0, 0.0));
            assert!((length(v) - 1.0).abs() < 1e-9);
            assert_vec3_close(v, Vec3::new(0.6, 0.8, 0.0), 1e-9);
        }

        #[test]
        fn normalize_zero_returns_unit_vector() {
            let v = normalize(Vec3::new(0.0, 0.0, 0.0));
            assert!((length(v) - 1.0).abs() < 1e-9);
        }
    }

    mod rotation {
        use super::*;

        fn rotate(v: Vec3, axis: Vec3, angle: f64) -> Vec3 {
            let mut pos = v;
            rotate_normalize_in_place(&mut pos, axis, angle);
            pos
        }

        #[test]
        fn rotate_x_around_z_by_90_gives_y() {
            let result = rotate(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), PI / 2.0);
            assert_vec3_close(result, Vec3::new(0.0, 1.0, 0.0), 1e-6);
        }

        #[test]
        fn rotate_x_around_z_by_180_gives_neg_x() {
            let result = rotate(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), PI);
            assert_vec3_close(result, Vec3::new(-1.0, 0.0, 0.0), 1e-6);
        }

        #[test]
        fn rotate_x_around_z_by_360_gives_x() {
            let result = rotate(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), 2.0 * PI);
            assert_vec3_close(result, Vec3::new(1.0, 0.0, 0.0), 1e-6);
        }

        #[test]
        fn rotate_around_own_axis_does_nothing() {
            let result = rotate(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), PI / 2.0);
            assert_vec3_close(result, Vec3::new(1.0, 0.0, 0.0), 1e-6);
        }

        #[test]
        fn preserves_unit_length() {
            let v = normalize(Vec3::new(1.0, 1.0, 1.0));
            let axis = normalize(Vec3::new(1.0, 2.0, 3.0));
            let result = rotate(v, axis, 1.234);
            assert!((length(result) - 1.0).abs() < 1e-9);
        }
    }

    mod tangent_basis {
        use super::*;

        #[test]
        fn returns_orthonormal_vectors() {
            let u = normalize(Vec3::new(1.0, 2.0, 3.0));
            let (e1, e2) = build_tangent_basis(u);

            assert!((length(e1) - 1.0).abs() < 1e-9);
            assert!((length(e2) - 1.0).abs() < 1e-9);
            assert!(dot(u, e1).abs() < 1e-9);
            assert!(dot(u, e2).abs() < 1e-9);
            assert!(dot(e1, e2).abs() < 1e-9);
        }

        #[test]
        fn works_for_y_axis() {
            let u = Vec3::new(0.0, 1.0, 0.0);
            let (e1, e2) = build_tangent_basis(u);
            assert!(dot(u, e1).abs() < 1e-9);
            assert!(dot(u, e2).abs() < 1e-9);
            assert!(dot(e1, e2).abs() < 1e-9);
        }

        #[test]
        fn works_for_all_axes() {
            for u in [
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ] {
                let (e1, e2) = build_tangent_basis(u);
                assert!(dot(u, e1).abs() < 1e-9, "e1 not orthogonal for {:?}", u);
                assert!(dot(u, e2).abs() < 1e-9, "e2 not orthogonal for {:?}", u);
                assert!((length(e1) - 1.0).abs() < 1e-9);
                assert!((length(e2) - 1.0).abs() < 1e-9);
            }
        }
    }
}
