use api::{Vector, Ray, Refl, Sphere};
use std::ops::{Add, AddAssign, Sub, SubAssign, Mul, MulAssign, Div, DivAssign};

impl Vector {
    pub const fn zero() -> Self {
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Add for Vector {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl AddAssign for Vector {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Sub for Vector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl SubAssign for Vector {
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
    }
}

impl Mul for Vector {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }
}

impl MulAssign for Vector {
    fn mul_assign(&mut self, other: Self) {
        self.x *= other.x;
        self.y *= other.y;
        self.z *= other.z;
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    fn mul(self, n: f64) -> Self {
        Self::new(self.x * n, self.y * n, self.z * n)
    }
}

impl MulAssign<f64> for Vector {
    fn mul_assign(&mut self, n: f64) {
        self.x *= n;
        self.y *= n;
        self.z *= n;
    }
}

impl Div<f64> for Vector {
    type Output = Self;

    fn div(self, n: f64) -> Self {
        Self::new(self.x / n, self.y / n, self.z / n)
    }
}

impl DivAssign<f64> for Vector {
    fn div_assign(&mut self, n: f64) {
        self.x /= n;
        self.y /= n;
        self.z /= n;
    }
}

impl Vector {
    pub fn norm(self) -> Self {
        self / (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn dot(self, other: Vector) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vector) -> Self {
        Self::new(self.y * other.z - self.z * other.y,
                  self.z * other.x - self.x * other.z,
                  self.x * other.y - self.y * other.x)
    }
}

pub struct Hit<'a> {
    pub pos: Vector,
    pub norm: Vector,
    pub emit: Vector,
    pub color: Vector,
    pub refl: &'a Refl,
}

impl<'a> Hit<'a> {
    pub const fn new(pos: Vector,
                     norm: Vector,
                     emit: Vector,
                     color: Vector,
                     refl: &'a Refl)
                     -> Self {
        Hit {
            pos: pos,
            norm: norm,
            emit: emit,
            color: color,
            refl: refl,
        }
    }
}

impl Sphere {
    pub fn intersect(&self, ray: Ray) -> Option<(f64, Hit)> {
        let op = self.p - ray.o;
        let eps = 1e-4;
        let b = op.dot(ray.d);
        let det = b * b - op.dot(op) + self.rad * self.rad;
        if det < 0.0 {
            return None;
        }

        let det = det.sqrt();
        let t = {
            let t = b - det;
            if t > eps {
                t
            } else {
                let t = b + det;
                if t > eps {
                    t
                } else {
                    return None;
                }
            }
        };

        let x = ray.o + (ray.d * t);
        let n = (x - self.p).norm();
        Some((t, Hit::new(x, n, self.e, self.c, &self.refl)))
    }
}
