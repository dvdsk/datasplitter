use futures::join;
use tokio::prelude::*;
use warp::{Filter, Buf, http::StatusCode, Rejection};
use reqwest;
use structopt::StructOpt;
use std::path::PathBuf;

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
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    
    let opt = Opt::from_args();
    let ports = Ports::from(opt.clone());

    // Turn ports into new filter
    let ports = warp::any().map(move || ports);

    let error = warp::path("post_error")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::aggregate())
        .and(ports)
        .and_then(handle_error);

    let data = warp::path("post_data")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::aggregate())
        .and(ports)
        .and_then(handle_data);

    let routes = warp::post().and(error.or(data));

    warp::serve(routes)
        .tls()
        .cert_path(opt.cert_path)
        .key_path(opt.key_path)
        .run(([0u8, 0, 0, 0], opt.port_self))
        .await;
    
    Ok(())
}

async fn handle_data(mut body: impl Buf, ports: Ports)
 -> Result<impl warp::Reply,Rejection> {

    let client = reqwest::Client::new();
    let resp_stable = client
        .post(&format!("https://127.0.0.1:{}/post_data", ports.stable))
        .body(body.to_bytes())
        .send();
    let resp_dev = client
        .post(&format!("https://127.0.0.1:{}/post_data", ports.dev))
        .body(body.to_bytes())
        .send();

    let (resp_stable, _resp_dev) = join!(resp_stable, resp_dev);

    match resp_stable {
        Ok(_resp) => Ok(StatusCode::OK),
        Err(e) => {
            if let Some(code) = e.status() {
                Ok(StatusCode::from_u16(code.into()).unwrap())
            } else {
                Ok(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

async fn handle_error(mut body: impl Buf, ports: Ports)
 -> Result<impl warp::Reply,Rejection> {

    let client = reqwest::Client::new();
    let resp_stable = client
        .post(&format!("https://127.0.0.1:{}/post_error", ports.stable))
        .body(body.to_bytes())
        .send();
    let resp_dev = client
        .post(&format!("https://127.0.0.1:{}/post_error", ports.dev))
        .body(body.to_bytes())
        .send();

    let (resp_stable, _resp_dev) = join!(resp_stable, resp_dev);

    Ok(match resp_stable {
        Ok(_resp) => StatusCode::OK,
        Err(e) => {
            if let Some(code) = e.status() {
                StatusCode::from_u16(code.into()).unwrap()
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    })
}