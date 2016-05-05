#[derive(Copy, Clone, RustcDecodable, RustcEncodable)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Vector { x: x, y: y, z: z }
    }
}

#[derive(Copy, Clone, RustcDecodable, RustcEncodable)]
pub struct Ray {
    pub o: Vector,
    pub d: Vector,
}

impl Ray {
    pub const fn new(o: Vector, d: Vector) -> Self {
        Ray { o: o, d: d }
    }
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub enum Refl {
    Diff,
    Spec,
    Refr,
    Mix(f64, Box<Refl>, Box<Refl>),
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Sphere {
    pub rad: f64,
    pub p: Vector,
    pub e: Vector,
    pub c: Vector,
    pub refl: Refl,
}

impl Sphere {
    pub const fn new(rad: f64, p: Vector, e: Vector, c: Vector, refl: Refl) -> Self {
        Sphere {
            rad: rad,
            p: p,
            e: e,
            c: c,
            refl: refl,
        }
    }
}

#[derive(Copy, Clone, RustcDecodable, RustcEncodable)]
pub struct Rectangle {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

impl Rectangle {
    pub const fn new(left: usize, top: usize, width: usize, height: usize) -> Self {
        Rectangle {
            left: left,
            top: top,
            width: width,
            height: height,
        }
    }
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Task {
    pub tile: Rectangle,
    pub callback: String,
}

impl Task {
    pub const fn new(tile: Rectangle, callback: String) -> Self {
        Task {
            tile: tile,
            callback: callback,
        }
    }
}

#[derive(Clone, RustcDecodable, RustcEncodable)]
pub struct Session {
    pub width: usize,
    pub height: usize,
    pub samples: usize,
    pub camera: Ray,
    pub scene: Vec<Sphere>,
}

impl Session {
    pub fn new(width: usize, height: usize, samples: usize, camera: Ray, scene: &[Sphere]) -> Self {
        Session {
            width: width,
            height: height,
            samples: samples,
            camera: camera,
            scene: scene.to_vec(),
        }
    }
}
