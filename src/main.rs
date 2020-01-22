use futures::join;
use reqwest;
use structopt::StructOpt;
use std::path::PathBuf;

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{NoClientAuth, ServerConfig};
use actix_web::{HttpServer,App, web};
use actix_web::web::{Data, Bytes};
use actix_web::{
	http::StatusCode,
	HttpResponse,
};

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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
    #[structopt(short = "l", long = "listen_on", default_value = "38972")]
    port_self: u16,

    #[structopt(short = "s", long = "stable_server_port", default_value = "443")]
    port_stable: u16,

    #[structopt(short = "d", long = "dev_server_port", default_value = "8443")]
    port_dev: u16,

    #[structopt(short = "k", long = "private_key", default_value = "keys/user.key")]
    key_path: PathBuf,

    #[structopt(short = "c", long = "signed_certificate", default_value = "keys/cert.cert")]
    cert_path: PathBuf,

    #[structopt(short = "i", long = "intermediate_certificate", default_value = "keys/intermediate.cert")]
    inter_cert_path: PathBuf,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    
    let opt = Opt::from_args();
    let ports = Ports::from(opt.clone());

    // Turn ports into new filter
    let tls_config = make_tls_config(
        opt.cert_path, opt.key_path, 
        opt.inter_cert_path);

    HttpServer::new(move || {
        let data = actix_web::web::Data::new(ports.clone());
        
        App::new()
            .app_data(data)
            .service(web::resource("/post_data").to(handle_data))
            .service(web::resource("/post_error").to(handle_error))
    })
    .bind_rustls(&format!("0.0.0.0:{}", opt.port_self), tls_config).unwrap()
    .run()
    .await
}

async fn handle_data(state: Data<Ports>, body: Bytes)
 -> HttpResponse {

    let ports = state;

    let client = reqwest::Client::new();
    let resp_stable = client
        .post(&format!("https://127.0.0.1:{}/post_data", ports.stable))
        .body(body.clone())
        .send();
    let resp_dev = client
        .post(&format!("https://127.0.0.1:{}/post_data", ports.dev))
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
        .post(&format!("https://127.0.0.1:{}/post_error", ports.stable))
        .body(body.clone())
        .send();
    let resp_dev = client
        .post(&format!("https://127.0.0.1:{}/post_error", ports.dev))
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

pub fn make_tls_config<P: AsRef<Path>+std::fmt::Debug>(cert_path: P, key_path: P, 
    intermediate_cert_path: P) 
-> rustls::ServerConfig{

	let mut tls_config = ServerConfig::new(NoClientAuth::new());
	let cert_file = &mut BufReader::new(File::open(&cert_path)
		.expect(&format!("could not open certificate file: {:?}", cert_path)));
	let intermediate_file = &mut BufReader::new(File::open(&intermediate_cert_path)
		.expect(&format!("could not open intermediate certificate file: {:?}", intermediate_cert_path)));
	let key_file = &mut BufReader::new(File::open(&key_path)
		.expect(&format!("could not open key file: {:?}", key_path)));

	let mut cert_chain = certs(cert_file).unwrap();
	cert_chain.push(certs(intermediate_file).unwrap().pop().unwrap());

	let mut key = pkcs8_private_keys(key_file).unwrap();

	tls_config
		.set_single_cert(cert_chain, key.pop().unwrap())
		.unwrap();
	tls_config
}