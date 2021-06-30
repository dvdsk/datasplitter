use futures::join;
use reqwest;
use structopt::StructOpt;

use actix_web::{HttpServer,App, web};
use actix_web::web::{Data, Bytes};
use actix_web::{
	http::StatusCode,
	HttpResponse,
};

#[derive(Clone, Copy)]
struct Ports {
    dev: u16,
    stable: u16,
}

impl From<Opt> for Ports {
    fn from(o: Opt) -> Ports {
        Ports {
            dev: o.port_dev,
            stable: o.port_stable,
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "datasplitter", about = "splits traffic on urls \
 \\post_error and \\post_data between two different ports")]
struct Opt {
    #[structopt(long = "port")]
    port_self: u16,

    #[structopt(long = "port-stable-server")]
    port_stable: u16,

    #[structopt(long = "port-dev-server")]
    port_dev: u16,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    
    let opt = Opt::from_args();
    let ports = Ports::from(opt.clone());

    // Turn ports into new filter
    HttpServer::new(move || {
        let data = actix_web::web::Data::new(ports.clone());
        
        App::new()
            .app_data(data)
            .service(web::resource("/post_data").to(handle_data))
            .service(web::resource("/post_error").to(handle_error))
    })
    .bind(&format!("0.0.0.0:{}", opt.port_self)).unwrap()
    .run()
    .await
}

async fn handle_data(state: Data<Ports>, body: Bytes)
 -> HttpResponse {

    let ports = state;

    let client = reqwest::Client::new();
    let resp_stable = client
        .post(&format!("http://127.0.0.1:{}/post_data", ports.stable))
        .body(body.clone())
        .send();
    let resp_dev = client
        .post(&format!("http://127.0.0.1:{}/post_data", ports.dev))
        .body(body)
        .send();

    let (resp_stable, _resp_dev) = join!(resp_stable, resp_dev);

    match resp_stable {
        Ok(_resp) => HttpResponse::Ok().finish(),
        Err(e) => {
            if let Some(code) = e.status() {
                HttpResponse::build(
                    StatusCode::from_u16(code.into())
                    .unwrap())
                .finish()
            } else {
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

async fn handle_error(state: Data<Ports>, body: Bytes)
 -> HttpResponse {

    let ports = state;

    let client = reqwest::Client::new();
    let resp_stable = client
        .post(&format!("http://127.0.0.1:{}/post_error", ports.stable))
        .body(body.clone())
        .send();
    let resp_dev = client
        .post(&format!("http://127.0.0.1:{}/post_error", ports.dev))
        .body(body)
        .send();

    let (resp_stable, _resp_dev) = join!(resp_stable, resp_dev);

    match resp_stable {
        Ok(_resp) => HttpResponse::Ok().finish(),
        Err(e) => {
            if let Some(code) = e.status() {
                HttpResponse::build(
                    StatusCode::from_u16(code.into())
                    .unwrap())
                .finish()
            } else {
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}
