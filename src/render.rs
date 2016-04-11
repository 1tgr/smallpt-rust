use radiance;
use rand::{Rng, SeedableRng, StdRng};
use scene::{Vector, Ray, Sphere};
use std::sync::mpsc::Sender;

fn clamp(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

fn to_int(x: f64) -> u8 {
    (clamp(x).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

#[derive(Copy, Clone)]
pub struct Rectangle {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

impl Rectangle {
    pub fn new(left: usize, top: usize, width: usize, height: usize) -> Self {
        Rectangle {
            left: left,
            top: top,
            width: width,
            height: height,
        }
    }
}

pub fn render<Work: Iterator<Item = Rectangle>>(scene: &[Sphere],
                                                cam: Ray,
                                                samps: usize,
                                                w: usize,
                                                h: usize,
                                                work: &mut Work,
                                                tx: &Sender<(Rectangle, Vec<u8>)>) {
    let mut xi = StdRng::new().unwrap();
    let cx = Vector::new(w as f64 * 0.5135 / h as f64, 0.0, 0.0);
    let cy = cx.cross(cam.d).norm() * 0.5135;
    for rect in work {
        let mut image = Vec::with_capacity(rect.width * 4 * rect.height);
        for y in rect.top..rect.top + rect.height {
            let y = h - y - 1;
            xi.reseed(&[y * y * y]);
            for x in rect.left..rect.left + rect.width {
                let mut c = Vector::zero();
                for sy in 0..2 {
                    for sx in 0..2 {
                        let mut r = Vector::zero();
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
                                    (((sy as f64 + 0.5 + dy) / 2.0 + y as f64) / (h as f64) -
                                     0.5) + cam.d;

                            let ray = Ray::new(cam.o + d * 140.0, d.norm());
                            r = r + radiance::radiance(&*scene, ray, 0, &mut xi) / samps as f64;
                        }

                        c = c + Vector::new(clamp(r.x), clamp(r.y), clamp(r.z)) / 4.0;
                    }
                }

                image.push(to_int(c.x));
                image.push(to_int(c.y));
                image.push(to_int(c.z));
                image.push(0);
            }
        }

        tx.send((rect, image)).unwrap();
    }
}
