use std::ops::{Add, Sub, Mul, Div};

#[derive(Copy, Clone)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector { x: x, y: y, z: z }
    }

    pub fn zero() -> Self {
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

impl Sub for Vector {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul for Vector {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    fn mul(self, n: f64) -> Self {
        Self::new(self.x * n, self.y * n, self.z * n)
    }
}

impl Div<f64> for Vector {
    type Output = Self;

    fn div(self, n: f64) -> Self {
        Self::new(self.x / n, self.y / n, self.z / n)
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

#[derive(Copy, Clone)]
pub struct Ray {
    pub o: Vector,
    pub d: Vector,
}

impl Ray {
    pub fn new(o: Vector, d: Vector) -> Self {
        Ray { o: o, d: d }
    }
}

#[derive(Copy, Clone)]
pub enum Refl {
    Diff,
    Spec,
    Refr,
}

#[derive(Copy, Clone)]
pub struct Hit {
    pub t: f64,
    pub pos: Vector,
    pub norm: Vector,
    pub emit: Vector,
    pub color: Vector,
    pub refl: Refl,
}

impl Hit {
    pub fn new(t: f64, pos: Vector, norm: Vector, emit: Vector, color: Vector, refl: Refl) -> Self {
        Hit {
            t: t,
            pos: pos,
            norm: norm,
            emit: emit,
            color: color,
            refl: refl,
        }
    }
}

pub struct Sphere {
    pub rad: f64,
    pub p: Vector,
    pub e: Vector,
    pub c: Vector,
    pub refl: Refl,
}

impl Sphere {
    pub fn new(rad: f64, p: Vector, e: Vector, c: Vector, refl: Refl) -> Self {
        Sphere {
            rad: rad,
            p: p,
            e: e,
            c: c,
            refl: refl,
        }
    }

    pub fn intersect(&self, ray: Ray) -> Option<Hit> {
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
        Some(Hit::new(t, x, n, self.e, self.c, self.refl))
    }
}
