extern crate cairo;
extern crate glib;
extern crate gtk;
extern crate rand;

use cairo::{Context, Format, ImageSurface};
use gtk::prelude::*;
use rand::{Rng, SeedableRng, StdRng};
use render::{Vector, Ray, Sphere};
use render::Refl::*;
use std::cell::RefCell;
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{Write, stderr};
use std::process;
use std::rc::Rc;
use std::result::Result;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread;

mod render;

fn clamp(x: f64) -> f64 {
    x.max(0.0).min(1.0)
}

fn to_int(x: f64) -> u8 {
    (clamp(x).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
}

fn render(scene: &[Sphere],
          cam: Ray,
          samps: usize,
          w: usize,
          h: usize,
          stride: usize,
          y0: usize,
          y1: usize,
          sender: Sender<(usize, Vec<u8>)>) {
    let mut xi = StdRng::new().unwrap();
    let cx = Vector::new((w as f64) * 0.5135 / (h as f64), 0.0, 0.0);
    let cy = cx.cross(cam.d).norm() * 0.5135;
    for y in y0..y1 {
        xi.reseed(&[y * y * y]);

        let mut line = Vec::with_capacity(stride);
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

            line.push(to_int(c.x));
            line.push(to_int(c.y));
            line.push(to_int(c.z));
            line.push(0);
        }

        sender.send((y, line)).unwrap();
    }
}

#[derive(Debug)]
struct AppError<'a>(&'a str);

impl<'a> AppError<'a> {
    pub fn new(desc: &'a str) -> Self {
        AppError(desc)
    }
}

impl<'a> fmt::Display for AppError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl<'a> Error for AppError<'a> {
    fn description(&self) -> &str {
        self.0
    }
}

fn run() -> Result<i32, Box<Error>> {
    try!(gtk::init().map_err(|()| AppError::new("Failed to initialise GTK")));
    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("smallpt");
    window.set_border_width(10);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

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
    let stride = w * 4;
    let threads = 4;
    let samps = env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(1);
    let cam = Ray::new(Vector::new(50.0, 52.0, 295.6),
                       Vector::new(0.0, -0.042612, -1.0).norm());
    let (tx, rx) = mpsc::channel();

    {
        let th = h / threads;
        for i in 0..threads {
            let scene = scene.clone();
            let tx = tx.clone();
            let y0 = th * i;
            let y1 = y0 + th;
            thread::Builder::new()
                .stack_size(8 * 1024 * 1024)
                .spawn(move || render(&*scene, cam, samps, w, h, stride, y0, y1, tx))
                .unwrap();
        }
    }

    let area = gtk::DrawingArea::new();
    area.set_size_request(w as i32, h as i32);
    window.add(&area);

    let surface = Rc::new(RefCell::new(ImageSurface::create(Format::Rgb24, w as i32, h as i32)));

    {
        let surface = surface.clone();
        area.connect_draw(move |_, cr| {
            let surface = surface.borrow();
            cr.set_source_surface(&*surface, 0.0, 0.0);
            cr.paint();
            Inhibit(false)
        });
    }

    {
        let surface = surface.clone();
        gtk::timeout_add(200, move || {
            while let Ok((y, line)) = rx.try_recv() {
                let y = h - y - 1;
                let line_surface = ImageSurface::create_for_data(line.into_boxed_slice(),
                                                                 |_| (),
                                                                 Format::Rgb24,
                                                                 w as i32,
                                                                 1,
                                                                 stride as i32);
                let surface = surface.borrow();
                let cr = Context::new(&*surface);
                cr.set_source_surface(&line_surface, 0.0, y as f64);
                cr.paint();
                area.queue_draw_area(0, y as i32, w as i32, 1);
            }

            Continue(true)
        });
    }

    window.show_all();
    gtk::main();
    Ok(0)
}

fn main() {
    process::exit(run().unwrap_or_else(|err| {
        writeln!(stderr(), "{}", err).unwrap();
        1
    }))
}
