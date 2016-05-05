use api::{Vector, Ray, Rectangle, Session, Task};
use cairo::{Context, Format, ImageSurface};
use flate2::read::GzDecoder;
use gtk;
use gtk::prelude::*;
use hyper::client::Client;
use hyper::header::ContentType;
use hyper::Url;
use iron::Iron;
use num_cpus;
use rand::StdRng;
use render;
use rustc_serialize::json;
use rustless::{Application, Api, Nesting};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::mem;
use std::rc::Rc;
use std::result::Result;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use super::{AppError, Args, WorkIterator, SCENE};

fn call_mut1<A, B, F: FnMut(A) -> B>(f: &mut F, a: A) -> B {
    f(a)
}

pub fn run(args: &Args) -> Result<i32, Box<Error>> {
    try!(gtk::init().map_err(|()| AppError::new("Failed to initialise GTK")));
    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    let title = "smallpt";
    window.set_title(title);
    window.set_border_width(10);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let session = Arc::new(Session::new(1024,
                                        768,
                                        args.flag_samples.unwrap_or(1),
                                        Ray::new(Vector::new(50.0, 52.0, 295.6),
                                                 Vector::new(0.0, -0.042612, -1.0).norm()),
                                        SCENE));

    let (tx_work, rx_work) = mpsc::channel();
    let (_tx_cancel, rx_cancel) = mpsc::channel();
    let (tx_images, rx_images) = mpsc::channel();
    let tx_images = Arc::new(Mutex::new(tx_images));
    let work = Arc::new(Mutex::new((rx_work, rx_cancel)));

    for _ in 0..args.flag_threads.unwrap_or_else(num_cpus::get) {
        let work = work.clone();
        let session = session.clone();
        thread::spawn(move || {
            let mut xi = StdRng::new().unwrap();
            for (tile, tx) in WorkIterator::new(&work) {
                render::render(&mut xi, &*session, tile, tx);
            }
        });
    }

    if !args.flag_agent.is_empty() {
        let tasks = Arc::new(Mutex::new(BTreeMap::new()));
        thread::spawn(clone!(tasks => move || {
            let api = Api::build(|api| {
                api.mount(Api::build(|api| {
                    api.post("response/:id", |endpoint| {
                        endpoint.handle(move |client, params| {
                            let id = params.find("id").unwrap().as_string().unwrap();
                            if let Some(tx) = tasks.lock().unwrap().get_mut(id) {
                                let mut compressed_image = Vec::new();
                                client.request.body_mut().read_to_end(&mut compressed_image).unwrap();

                                let mut d = GzDecoder::new(compressed_image.as_slice()).unwrap();
                                let mut image = Vec::new();
                                d.read_to_end(&mut image).unwrap();
                                call_mut1(tx, image)
                            }

                            client.text("OK".to_string())
                        })
                    });
                }));
            });

            let app = Application::new(api);
            Iron::new(app).http("0.0.0.0:4001").unwrap();
        }));

        let client = Arc::new(Client::new());
        for ref agent_url in args.flag_agent.iter() {
            let agent_url = try!(Url::parse(&agent_url));
            let session_url = try!(agent_url.join("session"));
            let mut session_id = String::new();
            client.post(session_url)
                  .header(ContentType::json())
                  .body(&json::encode(&*session).unwrap())
                  .send()
                  .unwrap()
                  .read_to_string(&mut session_id)
                  .unwrap();

            let task_url = try!(agent_url.join(&format!("session/{}/task", session_id)));
            println!("Sending tasks to {}", task_url);
            thread::spawn(clone!(tasks, client, work => move || {
                let work = WorkIterator::new(&work);
                for (tile, tx) in work {
                    let id = {
                        let mut tasks = tasks.lock().unwrap();
                        let id = tasks.len().to_string();
                        tasks.insert(id.clone(), tx);
                        id
                    };

                    let task = Task::new(tile, format!("http://localhost:4001/response/{}", id));
                    client.post(task_url.clone())
                          .header(ContentType::json())
                          .body(&json::encode(&task).unwrap())
                          .send()
                          .unwrap();
                }
            }));
        }
    }

    let w = session.width;
    let h = session.height;

    {
        let mut tiles = Vec::new();
        let mut y = 0;
        while y < h {
            let mut x = 0;
            while x < w {
                tiles.push(Rectangle::new(x, y, 32, 32));
                x += 32;
            }

            y += 32;
        }

        tiles.sort_by_key(|tile| {
            let tx = tile.left + tile.width / 2;
            let ty = tile.top + tile.height / 2;
            let dx = tx as isize - w as isize / 2;
            let dy = ty as isize - h as isize / 2;
            (dx * dx + dy * dy, tile.top, tile.left)
        });

        for tile in tiles {
            let tx_images = tx_images.clone();
            try!(tx_work.send((tile,
                               move |image| {
                tx_images.lock()
                         .unwrap()
                         .send((tile, image))
                         .unwrap()
            })));
        }
    }

    let area = gtk::DrawingArea::new();
    area.set_size_request(w as i32, h as i32);
    window.add(&area);

    let surface = Rc::new(RefCell::new(ImageSurface::create(Format::Rgb24, w as i32, h as i32)));
    area.connect_draw(clone!(surface => move |_, cr| {
        let surface = surface.borrow();
        cr.set_source_surface(&*surface, 0.0, 0.0);
        cr.paint();
        Inhibit(false)
    }));

    let window = Rc::new(RefCell::new(window));
    let mut total_pixels = 0;
    let mut total_time = 0;
    gtk::timeout_add(200,
                     clone!(surface, window => move || {
        let window = window.borrow();
        total_time += 200;
        while let Ok((rect, image)) = rx_images.try_recv() {
            let image_surface = ImageSurface::create_for_data(image.into_boxed_slice(),
                                                              mem::drop,
                                                              Format::Rgb24,
                                                              rect.width as i32,
                                                              rect.height as i32,
                                                              (rect.width * 4) as i32);

            let surface = surface.borrow();
            let cr = Context::new(&*surface);
            cr.set_source_surface(&image_surface, rect.left as f64, rect.top as f64);
            cr.paint();
            area.queue_draw_area(rect.left as i32, rect.top as i32, rect.width as i32, rect.height as i32);
            total_pixels += rect.width * rect.height;
        }

        let title = format!("{} ({} pixels/sec)", title, (1000 * total_pixels) / total_time);
        window.set_title(&title);
        Continue(true)
    }));

    window.borrow().show_all();
    gtk::main();
    Ok(0)
}
