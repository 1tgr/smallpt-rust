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

pub fn render<Work: Iterator<Item = usize>>(scene: &[Sphere],
                                            cam: Ray,
                                            samps: usize,
                                            w: usize,
                                            h: usize,
                                            stride: usize,
                                            work: &mut Work,
                                            tx: Sender<(usize, Vec<u8>)>) {
    let mut xi = StdRng::new().unwrap();
    let cx = Vector::new((w as f64) * 0.5135 / (h as f64), 0.0, 0.0);
    let cy = cx.cross(cam.d).norm() * 0.5135;
    for y in work {
        xi.reseed(&[y * y * y]);

        let mut line = Vec::with_capacity(stride);
        for x in 0..w {
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
                                (((sy as f64 + 0.5 + dy) / 2.0 + y as f64) / (h as f64) - 0.5) +
                                cam.d;

                        let ray = Ray::new(cam.o + d * 140.0, d.norm());
                        r = r + radiance::radiance(&*scene, ray, 0, &mut xi) / samps as f64;
                    }

                    c = c + Vector::new(clamp(r.x), clamp(r.y), clamp(r.z)) / 4.0;
                }
            }

            line.push(to_int(c.x));
            line.push(to_int(c.y));
            line.push(to_int(c.z));
            line.push(0);
        }

        tx.send((y, line)).unwrap();
    }
}
