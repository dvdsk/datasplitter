use futures::join;
use async_std;
use warp::{Filter, Buf, http::StatusCode, Rejection};
use reqwest;

#[derive(Clone, Copy)]
struct Ports {
    dev: u16,
    stable: u16,
}

#[async_std::main]
async fn main() {
    let port_self: u16 = 38972;
    let ports = Ports {
        dev: 8443,
        stable: 443,
    };

    // Turn ports into new filter
    let ports = warp::any().map(move || ports);

    let error = warp::path("/post_error")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::aggregate())
        .and(ports)
        .and_then(handle_error);

    let data = warp::path("/post_data")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::aggregate())
        .and(ports)
        .and_then(handle_data);

    let routes = warp::post().and(error.or(data));

    warp::serve(routes)
        .tls()
        .cert_path("keys/cert.pem")
        .key_path("keys/key.rsa")
        .run(([0u8, 0, 0, 0], port_self))
        .await;
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