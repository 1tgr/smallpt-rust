extern crate lodepng;
extern crate rand;

mod render;

use lodepng::RGB;
use rand::{Rng, SeedableRng, StdRng};
use render::{Vector, Ray, Sphere};
use render::Refl::*;
use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;

fn clamp(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

fn to_int(x: f64) -> u8 {
    (clamp(x).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

struct RenderShared {
    image: Vec<RGB<u8>>,
    progress: usize,
}

impl RenderShared {
    pub fn new(width: usize, height: usize) -> Self {
        RenderShared {
            image: vec![RGB { r: 0, g: 0, b: 0 }; width * height],
            progress: 0,
        }
    }
}

fn render(shared: &Mutex<RenderShared>,
          scene: &[Sphere],
          cam: Ray,
          samps: usize,
          w: usize,
          h: usize,
          y0: usize,
          y1: usize) {
    let mut line = Vec::with_capacity(w);
    let mut xi = StdRng::new().unwrap();
    let cx = Vector::new((w as f64) * 0.5135 / (h as f64), 0.0, 0.0);
    let cy = cx.cross(cam.d).norm() * 0.5135;
    for y in y0..y1 {
        {
            let mut shared = shared.lock().unwrap();
            let _ = write!(io::stderr(),
                           "\rRendering ({} spp) {:-3.2}%",
                           samps * 4,
                           (100.0 * shared.progress as f64) / h as f64);
            shared.progress += 1;
        }

        xi.reseed(&[y * y * y]);
        line.clear();
        for x in 0..w {
            let mut c = Vector::new(0.0, 0.0, 0.0);
            for sy in 0..2 {
                for sx in 0..2 {
                    let mut r = Vector::new(0.0, 0.0, 0.0);
                    for _ in 0..samps {
                        let r1 = 2.0 * xi.next_f64();
                        let r2 = 2.0 * xi.next_f64();
                        let dx = if r1 < 1.0 {
                            r1.sqrt() - 1.0
                        } else {
                            1.0 - (2.0 - r1).sqrt()
                        };
                        let dy = if r2 < 1.0 {
                            r2.sqrt() - 1.0
                        } else {
                            1.0 - (2.0 - r2).sqrt()
                        };
                        let d = cx *
                                (((sx as f64 + 0.5 + dx) / 2.0 + x as f64) / (w as f64) - 0.5) +
                                cy *
                                (((sy as f64 + 0.5 + dy) / 2.0 + y as f64) / (h as f64) - 0.5) +
                                cam.d;

                        let ray = Ray::new(cam.o + d * 140.0, d.norm());
                        r = r + render::radiance(&*scene, ray, 0, &mut xi) / samps as f64;
                    }

                    c = c + Vector::new(clamp(r.x), clamp(r.y), clamp(r.z)) / 4.0;
                }
            }

            line.push(RGB {
                r: to_int(c.x),
                g: to_int(c.y),
                b: to_int(c.z),
            });
        }

        let offset = (h - y - 1) * w;
        let mut shared = shared.lock().unwrap();
        for (i, pixel) in line.iter().enumerate() {
            shared.image[offset + i] = *pixel;
        }
    }
}

fn main() {
    let zero = Vector::new(0.0, 0.0, 0.0);

    // Scene: radius, position, emission, color, material
    let scene = Arc::new([Sphere::new(1e5,
                                      Vector::new(1e5 + 1.0, 40.8, 81.6),
                                      zero,
                                      Vector::new(0.75, 0.25, 0.25),
                                      Diff), // Left
                          Sphere::new(1e5,
                                      Vector::new(-1e5 + 99.0, 40.8, 81.6),
                                      zero,
                                      Vector::new(0.25, 0.25, 0.75),
                                      Diff), // Rght
                          Sphere::new(1e5,
                                      Vector::new(50.0, 40.8, 1e5),
                                      zero,
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Back
                          Sphere::new(1e5,
                                      Vector::new(50.0, 40.8, -1e5 + 170.0),
                                      zero,
                                      zero,
                                      Diff), // Frnt
                          Sphere::new(1e5,
                                      Vector::new(50.0, 1e5, 81.6),
                                      zero,
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Botm
                          Sphere::new(1e5,
                                      Vector::new(50.0, -1e5 + 81.6, 81.6),
                                      zero,
                                      Vector::new(0.75, 0.75, 0.75),
                                      Diff), // Top
                          Sphere::new(16.5,
                                      Vector::new(27.0, 16.5, 47.0),
                                      zero,
                                      Vector::new(0.999, 0.999, 0.999),
                                      Spec), // Mirr
                          Sphere::new(16.5,
                                      Vector::new(73.0, 16.5, 78.0),
                                      zero,
                                      Vector::new(0.999, 0.999, 0.999),
                                      Refr), // Glas
                          Sphere::new(600.0,
                                      Vector::new(50.0, 681.6 - 0.27, 81.6),
                                      Vector::new(12.0, 12.0, 12.0),
                                      zero,
                                      Diff) /* Lite */]);

    let w = 1024;
    let h = 768;
    let shared = Arc::new(Mutex::new(RenderShared::new(w, h)));
    let samps = env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(1);
    let cam = Ray::new(Vector::new(50.0, 52.0, 295.6),
                       Vector::new(0.0, -0.042612, -1.0).norm());

    {
        let mut threads = Vec::with_capacity(4);
        for i in 0..threads.capacity() {
            let scene = scene.clone();
            let shared = shared.clone();
            let th = h / threads.capacity();
            let y0 = th * i;
            let y1 = th * (i + 1);
            threads.push(thread::Builder::new()
                             .stack_size(8 * 1024 * 1024)
                             .spawn(move || render(&*shared, &*scene, cam, samps, w, h, y0, y1))
                             .unwrap());
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }

    let shared = shared.lock().unwrap();
    lodepng::encode24_file("image.png", &shared.image, w, h).unwrap();
}
