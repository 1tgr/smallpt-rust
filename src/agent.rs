use api::{Session, Task};
use flate2::Compression;
use flate2::write::GzEncoder;
use hyper::client::Client;
use iron::Iron;
use num_cpus;
use rand::StdRng;
use render;
use rustc_serialize::Decodable;
use rustc_serialize::json::Decoder;
use rustless::{Application, Api, Nesting};
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use super::{Args, WorkIterator};

pub fn run(args: &Args) -> Result<i32, Box<Error>> {
    let (tx_work, rx_work) = mpsc::channel();
    let (_tx_cancel, rx_cancel) = mpsc::channel();
    let tx_work = Mutex::new(tx_work);
    let work = Arc::new(Mutex::new((rx_work, rx_cancel)));
    let sessions = Arc::new(Mutex::new(BTreeMap::new()));

    for _ in 0..args.flag_threads.unwrap_or_else(num_cpus::get) {
        let work = work.clone();
        thread::spawn(move || {
            let mut xi = StdRng::new().unwrap();
            for (session, tile, tx) in WorkIterator::new(&work) {
                let session: Arc<Session> = session;
                render::render(&mut xi, &*session, tile, tx);
            }
        });
    }

    let api = Api::build(|api| {
        api.mount(Api::build(|api| {
            api.post("session", |endpoint| {
                let sessions = sessions.clone();
                endpoint.handle(move |client, params| {
                    let session = Session::decode(&mut Decoder::new(params.clone())).unwrap();
                    let mut sessions = sessions.lock().unwrap();
                    let session_id = sessions.len().to_string();
                    sessions.insert(session_id.clone(), Arc::new(session));
                    client.text(session_id)
                })
            });

            api.post("session/:session_id/task", |endpoint| {
                let sessions = sessions.clone();
                endpoint.handle(move |client, params| {
                    let session_id = params.find("session_id").unwrap().as_string().unwrap();
                    let session = sessions.lock().unwrap().get(session_id).unwrap().clone();
                    let task = Task::decode(&mut Decoder::new(params.clone())).unwrap();
                    tx_work.lock()
                           .unwrap()
                           .send((session,
                                  task.tile,
                                  move |image: Vec<u8>| {
                               let mut e = GzEncoder::new(Vec::new(), Compression::Default);
                               e.write_all(&image).unwrap();

                               let compressed_image = e.finish().unwrap();
                               let _ = Client::new()
                                           .post(&task.callback)
                                           .body(compressed_image.as_slice())
                                           .send();
                           }))
                           .unwrap();
                    client.text("OK".to_string())
                })
            });
        }));
    });

    let app = Application::new(api);
    try!(Iron::new(app).http("0.0.0.0:4000"));
    Ok(0)
}
